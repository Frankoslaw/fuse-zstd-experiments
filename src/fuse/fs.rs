use std::{
    io,
    sync::Arc,
    time::{Duration, SystemTime},
};

use fuser::{Errno, FileAttr, FileType, Filesystem, Generation, INodeNo, KernelConfig};

use crate::block::BlockDevice;

const DISK_IMG_INODE: u64 = 2;
const HUGE_BLKSIZE: u32 = 2 * 1024 * 1024;

pub struct FuseImageFS<B: BlockDevice> {
    pub backend: Arc<B>,
}

impl<B: BlockDevice> FuseImageFS<B> {
    fn file_attr(&self, req: &fuser::Request) -> FileAttr {
        let now = SystemTime::now();

        FileAttr {
            ino: INodeNo(DISK_IMG_INODE),
            size: self.backend.len(),
            blocks: self.backend.len().div_ceil(512),
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

impl<B: BlockDevice + 'static> Filesystem for FuseImageFS<B> {
    fn init(&mut self, _req: &fuser::Request, config: &mut KernelConfig) -> io::Result<()> {
        // TODO: figure out optimal and allowed value
        let max_readahead = 128 * 1024;

        if let Err(max) = config.set_max_readahead(max_readahead) {
            log::warn!("readahead rejected, using kernel max={}", max)
        }

        let max_write = 128 * 1024;

        if let Err(max) = config.set_max_write(max_write) {
            log::warn!("max_write rejected, using kernel max={}", max)
        }

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

        let mut buf = vec![0u8; size as usize];

        match self.backend.read_at(offset, &mut buf) {
            Ok(n) => reply.data(&buf[..n]),
            Err(_) => reply.error(Errno::from_i32(libc::EIO)),
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
}
