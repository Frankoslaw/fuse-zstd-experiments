use std::{fs::File, io, sync::Mutex};

use zstd_seekable::Seekable;

use crate::block::BlockDevice;

pub struct ZstdSeekableBackend {
    inner: Mutex<Seekable<'static, File>>,
}

impl ZstdSeekableBackend {
    pub fn from_zstd_file(path: &str) -> io::Result<Self> {
        let file = File::open(path)?;

        let seekable = zstd_seekable::Seekable::init(Box::new(file))
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(Self {
            inner: Mutex::new(seekable),
        })
    }
}

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
