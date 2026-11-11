use std::{fs::File, io, path::Path, sync::Mutex};

use anyhow::anyhow;
use zstd_seekable::Seekable;

use crate::{
    block::BlockDevice,
    helper::{is_zst_file, is_zst_seekable},
};

pub struct ZstdSeekableBackend {
    inner: Mutex<Seekable<'static, File>>,
}

impl ZstdSeekableBackend {
    pub fn from_zstd_file(path: &Path) -> anyhow::Result<Self> {
        let file = File::open(path)?;

        if !is_zst_file(path)? || !is_zst_seekable(path)? {
            return Err(anyhow!(
                "ZstdSeekableBackend only supports seekable .zst files"
            ));
        }

        let seekable = zstd_seekable::Seekable::init(Box::new(file))
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(Self {
            inner: Mutex::new(seekable),
        })
    }
}

// TODO: make this per thread to speedup fuse and decrease lock congestion
// TODO: move to file per backend
impl BlockDevice for ZstdSeekableBackend {
    fn len(&self) -> u64 {
        let guard = self.inner.lock().unwrap();
        guard.get_frame_decompressed_offset(guard.get_num_frames() - 1)
            + guard.get_frame_decompressed_size(guard.get_num_frames() - 1) as u64
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        let mut guard = self.inner.lock().unwrap();

        match guard.decompress(buf, offset) {
            Ok(n) => Ok(n),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
}
