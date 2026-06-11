#![allow(unused)]
use std::{fs, io, path::Path};

use anyhow::anyhow;

use crate::{block::BlockDevice, helper::is_zst_file};

pub struct ZstdMemoryBackend {
    pub data: Vec<u8>,
}

impl ZstdMemoryBackend {
    pub fn from_zstd_file(path: &Path) -> anyhow::Result<Self> {
        let file = fs::read(path)?;

        if !is_zst_file(path)? {
            return Err(anyhow!("ZstdMemoryBackend only supports .zst files"));
        }

        let data = zstd::stream::decode_all(io::Cursor::new(file))?;

        unsafe {
            libc::madvise(data.as_ptr() as *mut _, data.len(), libc::MADV_HUGEPAGE);
        }

        Ok(Self { data })
    }
}

impl BlockDevice for ZstdMemoryBackend {
    fn len(&self) -> u64 {
        self.data.len() as u64
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        let off = offset as usize;

        if off >= self.data.len() {
            return Ok(0);
        }

        let end = (off + buf.len()).min(self.data.len());
        let slice = &self.data[off..end];

        buf[..slice.len()].copy_from_slice(slice);
        Ok(slice.len())
    }
}
