use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

use zstd_seekable::SeekableCStream;

const FORCE_COMPRESS: bool = true;

pub fn ensure_seekable_zstd(input: &str, output: &str) -> io::Result<()> {
    if !FORCE_COMPRESS && Path::new(output).exists() {
        log::info!("Seekable file already exists: {}", output);
        return Ok(());
    }

    log::info!("Building SEEKABLE zstd: {} -> {}", input, output);

    let mut input = File::open(input)?;
    let mut output = File::create(output)?;

    // 🔥 frame size = key parameter for seek performance
    let frame_size = 4 * 1024 * 1024;

    let mut cstream =
        SeekableCStream::new(3, frame_size).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // ⚠️ YOU choose buffer sizes manually
    let mut in_buf = vec![0u8; 1 * 1024 * 1024]; // 1MB input chunks
    let mut out_buf = vec![0u8; 1 * 1024 * 1024 + 1024]; // safe headroom

    loop {
        let n = input.read(&mut in_buf)?;
        if n == 0 {
            break;
        }

        let mut consumed = 0;

        while consumed < n {
            let (out_len, in_pos) = cstream
                .compress(&mut out_buf, &in_buf[consumed..n])
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            consumed += in_pos;

            output.write_all(&out_buf[..out_len])?;
        }
    }

    let final_size = cstream
        .end_stream(&mut out_buf)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    output.write_all(&out_buf[..final_size])?;
    output.flush()?;

    log::info!("Seekable zstd image created successfully");

    Ok(())
}
