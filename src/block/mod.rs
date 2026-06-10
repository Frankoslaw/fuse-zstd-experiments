pub mod memory;

use std::io;

pub use memory::ZstdMemoryBackend;

pub trait BlockDevice: Send + Sync {
    fn len(&self) -> u64;

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize>;
}
