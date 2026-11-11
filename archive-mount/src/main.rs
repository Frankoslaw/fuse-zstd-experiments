use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
use fuser::MountOption;
use std::io;
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
    if let Some(shell) = completion_shell()? {
        generate(
            shell,
            &mut Args::command(),
            "archive-mount",
            &mut io::stdout(),
        );
        return Ok(());
    }

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

fn completion_shell() -> anyhow::Result<Option<Shell>> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--generate-completion" {
            let shell = args.next().ok_or_else(|| {
                anyhow::anyhow!("--generate-completion requires bash, fish, or zsh")
            })?;
            let shell = shell
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("completion shell name must be valid UTF-8"))?
                .parse::<Shell>()
                .map_err(|_| {
                    anyhow::anyhow!("unsupported completion shell; use bash, fish, or zsh")
                })?;
            if !matches!(shell, Shell::Bash | Shell::Fish | Shell::Zsh) {
                return Err(anyhow::anyhow!(
                    "unsupported completion shell; use bash, fish, or zsh"
                ));
            }
            return Ok(Some(shell));
        }
    }
    Ok(None)
}
