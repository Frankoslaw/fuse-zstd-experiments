use std::{
    fs, io,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};

use fuser::{
    Config, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo,
    KernelConfig, MountOption, SessionACL,
};

const DISK_IMG_INODE: u64 = 2;
const HUGE_BLKSIZE: u32 = 2 * 1024 * 1024;

struct RamDiskFs {
    image: Arc<Vec<u8>>,
}

impl RamDiskFs {
    fn file_attr(&self, req: &fuser::Request) -> FileAttr {
        let now = SystemTime::now();

        FileAttr {
            ino: INodeNo(DISK_IMG_INODE),
            size: self.image.len() as u64,
            blocks: (self.image.len() as u64).div_ceil(512),
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileType::RegularFile,
            perm: 0o444,
            nlink: 1,
            uid: req.uid(),
            gid: req.gid(),
            rdev: 0,
            blksize: HUGE_BLKSIZE,
            flags: 0,
        }
    }
}

impl Filesystem for RamDiskFs {
    fn init(&mut self, _req: &fuser::Request, config: &mut KernelConfig) -> io::Result<()> {
        // TODO: Proper error handling
        let _= config.set_max_readahead(16 * 1024 * 1024);
        // TODO: Does this option even matter for anything
        let _ = config.set_max_write(16 * 1024 * 1024);
        
        Ok(())
    }

    fn lookup(
        &self,
        req: &fuser::Request,
        parent: INodeNo,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        if parent == INodeNo::ROOT && name == "disk.img" {
            reply.entry(&Duration::from_secs(1), &self.file_attr(req), Generation(0));
        } else {
            reply.error(Errno::from_i32(libc::ENOENT));
        }
    }

    fn getattr(
        &self,
        req: &fuser::Request,
        ino: INodeNo,
        _fh: Option<fuser::FileHandle>,
        reply: fuser::ReplyAttr,
    ) {
        let now = SystemTime::now();

        match ino {
            INodeNo::ROOT => {
                reply.attr(
                    &Duration::from_secs(1),
                    &FileAttr {
                        ino: INodeNo::ROOT,
                        size: 0,
                        blocks: 0,
                        atime: now,
                        mtime: now,
                        ctime: now,
                        crtime: now,
                        kind: FileType::Directory,
                        perm: 0o755,
                        nlink: 2,
                        uid: req.uid(),
                        gid: req.gid(),
                        rdev: 0,
                        blksize: HUGE_BLKSIZE,
                        flags: 0,
                    },
                );
            }
            INodeNo(DISK_IMG_INODE) => {
                reply.attr(&Duration::from_secs(1), &self.file_attr(req));
            }
            _ => reply.error(Errno::from_i32(libc::ENOENT)),
        }
    }

    fn read(
        &self,
        _req: &fuser::Request,
        ino: INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        size: u32,
        _flags: fuser::OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: fuser::ReplyData,
    ) {
        if ino != INodeNo(DISK_IMG_INODE) {
            reply.error(Errno::from_i32(libc::ENOENT));
            return;
        }

        let data = &self.image;

        let start = offset as usize;

        if start >= data.len() {
            reply.data(&[]);
            return;
        }

        let end = std::cmp::min(start + size as usize, data.len());

        reply.data(&data[start..end]);
    }

    fn readdir(
        &self,
        _req: &fuser::Request,
        ino: INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        mut reply: fuser::ReplyDirectory,
    ) {
        if ino != INodeNo::ROOT {
            reply.error(Errno::from_i32(libc::ENOENT));
            return;
        }

        let entries = [
            (INodeNo::ROOT, FileType::Directory, "."),
            (INodeNo::ROOT, FileType::Directory, ".."),
            (INodeNo(DISK_IMG_INODE), FileType::RegularFile, "disk.img"),
        ];

        for (i, entry) in entries.iter().enumerate().skip(offset as usize) {
            if reply.add(entry.0, (i + 1) as u64, entry.1, entry.2) {
                break;
            }
        }

        reply.ok();
    }

    fn access(
        &self,
        _req: &fuser::Request,
        ino: INodeNo,
        mask: fuser::AccessFlags,
        reply: fuser::ReplyEmpty,
    ) {
        match ino {
            INodeNo::ROOT | INodeNo(DISK_IMG_INODE) => {
                let write_bits = libc::W_OK;

                if (mask.bits() & write_bits) != 0 {
                    reply.error(Errno::from_i32(libc::EROFS));
                } else {
                    reply.ok();
                }
            }
            _ => reply.error(Errno::from_i32(libc::ENOENT)),
        }
    }

    fn open(
        &self,
        _req: &fuser::Request,
        ino: INodeNo,
        flags: fuser::OpenFlags,
        reply: fuser::ReplyOpen,
    ) {
        if ino != INodeNo(DISK_IMG_INODE) {
            reply.error(Errno::from_i32(libc::ENOENT));
            return;
        }

        let accmode = flags.0 & libc::O_ACCMODE;

        if accmode != libc::O_RDONLY {
            reply.error(Errno::from_i32(libc::EROFS));
            return;
        }

        reply.opened(FileHandle(0), FopenFlags::FOPEN_DIRECT_IO);
    }
}

fn main() -> anyhow::Result<()> {
    if let Err(_) = std::env::var("RUST_LOG") {
        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }
    }
    pretty_env_logger::init();

    let img_path = std::env::args().nth(1).expect("usage: fzi <disk.img.zst>");

    log::info!("Loading {img_path}");

    let compressed = fs::read(&img_path)?;

    let image = zstd::stream::decode_all(io::Cursor::new(compressed))?;
    unsafe {
        libc::madvise(image.as_ptr() as *mut _, image.len(), libc::MADV_HUGEPAGE);
    };

    println!("Decompressed image: {} MiB", image.len() / 1024 / 1024);

    let fs = RamDiskFs {
        image: Arc::new(image),
    };

    let mount_point = PathBuf::from("./test-mount");

    fs::create_dir_all(&mount_point)?;

    let mut config = Config::default();
    config.mount_options.push(MountOption::AutoUnmount);
    config.mount_options.push(MountOption::RO);
    config.mount_options.push(MountOption::FSName(
        "fuse-zstd-img".into()
    ));
    config.acl = SessionACL::All;
    config.n_threads = Some(num_cpus::get());
    config.clone_fd = true;

    fuser::mount2(fs, &mount_point, &config)?;

    Ok(())
}
