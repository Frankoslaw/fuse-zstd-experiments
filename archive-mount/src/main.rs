use clap::Parser;
use fuser::MountOption;
use std::{path::PathBuf, sync::Arc};

use crate::{
    block::{BlockDevice, seekable::ZstdSeekableBackend},
    fuse::{FuseImageFS, default_config},
};

mod block;
mod fuse;
mod helper;

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Tool to mount zstd seekable files as fuse filesystems"
)]
struct Args {
    file: PathBuf,

    #[arg(short, long, default_value = "./.archive-mount")]
    mount_point: PathBuf,

    #[arg(long, default_value_t = true)]
    auto_create_mount: bool,

    #[arg(long, default_value_t = true)]
    auto_unmount: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if std::env::var("RUST_LOG").is_err() {
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }
    pretty_env_logger::init();

    log::info!("Loading seekable image: {}", args.file.to_string_lossy());

    let backend = Arc::new(ZstdSeekableBackend::from_zstd_file(&args.file)?);

    log::info!(
        "Decompressed image size: {} MiB",
        backend.len() / 1024 / 1024
    );

    let fs = FuseImageFS { backend };

    if args.auto_create_mount {
        std::fs::create_dir_all(&args.mount_point)?;
    }

    // TODO: move to builder pattern
    let mut config = default_config();
    if args.auto_unmount {
        config.mount_options.push(MountOption::AutoUnmount);
    }

    fuser::mount2(fs, &args.mount_point, &config)?;

    return Ok(());
}
