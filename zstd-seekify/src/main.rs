use std::{
    fs::File,
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use indicatif_log_bridge::LogWrapper;
use log::LevelFilter;

use crate::helper::{is_zst_file, is_zst_seekable};

mod helper;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Tool to re-encode files or zstd archives into Meta's seekable zstd variant"
)]
struct Args {
    file: PathBuf,

    #[arg(short, long, default_value_t = 3)]
    level: i32,

    #[arg(short, long, default_value_t = 4 * 1024 * 1024)]
    frame_size: usize,

    #[arg(short, long, default_value_t = 16 * 1024 * 1024)]
    input_buffer: usize,

    #[arg(short, long)]
    quiet: bool,
}

fn compress_seekable(
    in_path: &Path,
    out_path: &Path,
    level: usize,
    frame_size: usize,
    input_buffer_size: usize,
    pb: &ProgressBar,
) -> anyhow::Result<()> {
    let mut in_file = File::open(in_path)?;
    let out_file = File::create(out_path)?;

    let mut out_writer = BufWriter::with_capacity(4 * 1024 * 1024, out_file);

    let mut cstream = zstd_seekable::SeekableCStream::new(level, frame_size)
        .map_err(|_| anyhow!("Failed to initialize SeekableCStream"))?;

    let output_buffer_size = input_buffer_size + 1024;
    let mut out_buf = vec![0u8; output_buffer_size];
    let mut in_buf = vec![0u8; input_buffer_size];

    loop {
        let bytes_read = in_file.read(&mut in_buf)?;
        if bytes_read == 0 {
            break;
        }

        pb.inc(bytes_read as u64);

        let mut pos = 0;
        while pos < bytes_read {
            let (c_out, c_in) = cstream
                .compress(&mut out_buf, &in_buf[pos..bytes_read])
                .map_err(|_| anyhow!("Compression encountered an error"))?;

            if c_in == 0 && c_out == 0 {
                break;
            }

            pos += c_in;
            out_writer.write_all(&out_buf[..c_out])?;
        }
    }

    let c_out = cstream
        .end_stream(&mut out_buf)
        .map_err(|_| anyhow!("Failed to end seekable stream successfully"))?;

    if c_out > 0 {
        out_writer.write_all(&out_buf[..c_out])?;
    }

    out_writer.flush()?;

    Ok(())
}

fn recompress_seekable(
    in_path: &Path,
    out_path: &Path,
    level: usize,
    frame_size: usize,
    input_buffer_size: usize,
    pb: &ProgressBar,
) -> anyhow::Result<()> {
    let mut in_file = File::open(in_path)?;
    let out_file = File::create(out_path)?;

    let mut out_writer = BufWriter::with_capacity(4 * 1024 * 1024, out_file);

    let mut cstream = zstd_seekable::SeekableCStream::new(level, frame_size)
        .map_err(|_| anyhow!("Failed to initialize SeekableCStream"))?;
    let mut dstream =
        zstd_seekable::DStream::new().map_err(|_| anyhow!("Failed to initialize DStream"))?;

    let output_buffer_size = input_buffer_size + 1024;
    let mut out_buf = vec![0u8; output_buffer_size];
    let mut in_buf = vec![0u8; input_buffer_size];
    let mut compressed_in = vec![0u8; input_buffer_size];

    loop {
        let bytes_read = in_file.read(&mut compressed_in)?;
        if bytes_read == 0 {
            break;
        }

        pb.inc(bytes_read as u64);

        let mut c_pos = 0;
        while c_pos < bytes_read {
            let (d_out, d_in) = dstream
                .decompress(&mut in_buf, &compressed_in[c_pos..bytes_read])
                .map_err(|_| anyhow!("Decompression encountered an error"))?;

            if d_in == 0 && d_out == 0 {
                break;
            }

            c_pos += d_in;

            if d_out > 0 {
                let mut e_pos = 0;
                while e_pos < d_out {
                    let (c_out, c_in) = cstream
                        .compress(&mut out_buf, &in_buf[e_pos..d_out])
                        .map_err(|_| anyhow!("Compression encountered an error"))?;

                    if c_in == 0 && c_out == 0 {
                        break;
                    }

                    e_pos += c_in;
                    out_writer.write_all(&out_buf[..c_out])?;
                }
            }
        }
    }

    let c_out = cstream
        .end_stream(&mut out_buf)
        .map_err(|_| anyhow!("Failed to end seekable stream successfully"))?;

    if c_out > 0 {
        out_writer.write_all(&out_buf[..c_out])?;
    }

    out_writer.flush()?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let log_level = if args.quiet {
        LevelFilter::Off
    } else {
        LevelFilter::Info
    };
    let logger = pretty_env_logger::formatted_builder()
        .filter_level(log_level)
        .build();
    let multi = MultiProgress::new();

    LogWrapper::new(multi.clone(), logger)
        .try_init()
        .map_err(|e| anyhow!("Failed to initialize logger/indicatif bridge: {}", e))?;

    let is_zst = is_zst_file(&args.file)?;
    if is_zst {
        if is_zst_seekable(&args.file)? {
            return Err(anyhow!(
                "File seems to be already in seekable format, exiting"
            ));
        }
    }

    let file_name = args
        .file
        .file_name()
        .ok_or_else(|| anyhow!("Invalid filename provided"))?
        .to_string_lossy();

    let base_name = file_name.split('.').next().unwrap_or("output");

    let mut out_path = args.file.clone();
    out_path.set_file_name(format!("{}.seek.zst", base_name));

    let file_size = std::fs::metadata(&args.file)?.len();
    let pb = if args.quiet {
        ProgressBar::hidden()
    } else {
        let p = multi.add(ProgressBar::new(file_size));
        p.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        p
    };

    log::info!("Starting re-encode for: {:?}", args.file);

    let level = args.level as usize;

    if is_zst {
        recompress_seekable(
            &args.file,
            &out_path,
            level,
            args.frame_size,
            args.input_buffer,
            &pb,
        )?;
    } else {
        compress_seekable(
            &args.file,
            &out_path,
            level,
            args.frame_size,
            args.input_buffer,
            &pb,
        )?;
    }

    pb.finish_with_message("Encoding complete");

    log::info!("Successfully re-encoded to: {:?}", out_path);

    Ok(())
}
