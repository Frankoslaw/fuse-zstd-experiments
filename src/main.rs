use std::{path::PathBuf, sync::Arc};

use crate::{
    block::{BlockDevice, seekable::ZstdSeekableBackend},
    fuse::{FuseImageFS, default_config},
    helper::ensure_seekable_zstd,
};

pub mod block;
pub mod fuse;
pub mod helper;

fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }

    pretty_env_logger::init();

    let img_path = std::env::args()
        .nth(1)
        .expect("usage: fzi <disk.img | disk.img.zst>");

    let path = std::path::PathBuf::from(&img_path);

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let file_str = img_path.as_str();

    // ------------------------------------------------------------
    // MODE 1: RAW IMAGE → BUILD ZST AND EXIT
    // ------------------------------------------------------------
    if file_str.ends_with(".img") {
        let output = format!("{}.zst", file_str);

        log::info!("Raw image detected: {}", file_str);
        log::info!("Building seekable zstd: {}", output);

        ensure_seekable_zstd(file_str, &output)?;

        log::info!("Done. Exiting after build.");
        return Ok(());
    }

    // ------------------------------------------------------------
    // MODE 2: SEEKABLE IMAGE → MOUNT
    // ------------------------------------------------------------
    if ext == "zst" {
        log::info!("Loading seekable image: {}", file_str);

        let backend = Arc::new(ZstdSeekableBackend::from_zstd_file(file_str)?);

        log::info!(
            "Decompressed image size: {} MiB",
            backend.len() / 1024 / 1024
        );

        let fs = FuseImageFS { backend };

        let mount_point = PathBuf::from("./test-mount");
        std::fs::create_dir_all(&mount_point)?;

        let config = default_config();

        fuser::mount2(fs, &mount_point, &config)?;

        return Ok(());
    }

    // ------------------------------------------------------------
    // INVALID INPUT
    // ------------------------------------------------------------
    anyhow::bail!("Unsupported file type: {}", file_str);
}
