use std::{path::PathBuf, sync::Arc};

use crate::{
    block::{BlockDevice, ZstdMemoryBackend},
    fuse::{FuseImageFS, default_config},
};

pub mod block;
pub mod fuse;

fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }

    pretty_env_logger::init();

    let img_path = std::env::args().nth(1).expect("usage: fzi <disk.img.zst>");

    log::info!("Loading backend from {}", img_path);

    // backend owns decompression now
    let backend = Arc::new(ZstdMemoryBackend::from_zstd_file(&img_path)?);

    log::info!("Decompressed image: {} MiB", backend.len() / 1024 / 1024);

    let fs = FuseImageFS { backend };

    let mount_point = PathBuf::from("./test-mount");
    std::fs::create_dir_all(&mount_point)?;

    let config = default_config();

    fuser::mount2(fs, &mount_point, &config)?;

    Ok(())
}
