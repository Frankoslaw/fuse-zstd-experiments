use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};
use std::{fs, time::SystemTime};

use fuser::{
    Config, Errno, FileAttr, FileType, Filesystem, Generation, INodeNo, MountOption, SessionACL,
};

const TIME_FILE_INODE: u64 = 2;

struct TimeFs;

impl Filesystem for TimeFs {
    fn lookup(
        &self,
        req: &fuser::Request,
        parent: fuser::INodeNo,
        name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        if parent == INodeNo::ROOT && name == "test-time" {
            let now = SystemTime::now();
            let attr = FileAttr {
                ino: INodeNo(TIME_FILE_INODE),
                size: 1024,
                blocks: 1,
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
                blksize: 512,
                flags: 0,
            };
            reply.entry(&Duration::ZERO, &attr, Generation(0));
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
                let attr = FileAttr {
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
                    blksize: 512,
                    flags: 0,
                };
                reply.attr(&Duration::ZERO, &attr);
            }
            INodeNo(TIME_FILE_INODE) => {
                let attr = FileAttr {
                    ino: INodeNo(TIME_FILE_INODE),
                    size: 1024,
                    blocks: 1,
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
                    blksize: 512,
                    flags: 0,
                };
                reply.attr(&Duration::ZERO, &attr);
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
        if ino == INodeNo(TIME_FILE_INODE) {
            let now = SystemTime::now();
            let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();

            let time_str = format!("Current Unix Timestamp: {}\n", duration.as_secs());
            let bytes = time_str.as_bytes();

            let offset = offset as usize;
            if offset < bytes.len() {
                let end = std::cmp::min(offset + size as usize, bytes.len());
                reply.data(&bytes[offset..end]);
            } else {
                reply.data(&[]);
            }
        } else {
            reply.error(Errno::from_i32(libc::ENOENT));
        }
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

        let entries = vec![
            (INodeNo::ROOT, FileType::Directory, "."),
            (INodeNo::ROOT, FileType::Directory, ".."),
            (INodeNo(TIME_FILE_INODE), FileType::RegularFile, "test-time"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(entry.0, (i + 1) as u64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }
}
// struct NullFS;

// impl Filesystem for NullFS {}

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let mount_point: PathBuf = PathBuf::from("./test-mount");

    fs::create_dir_all(&mount_point)?;

    let mut config: Config = Config::default();
    config.mount_options.push(MountOption::AutoUnmount);
    config.acl = SessionACL::All;
    config.n_threads = Some(1);
    config.clone_fd = true;

    fuser::mount2(TimeFs, &mount_point, &config)?;

    Ok(())
}
