use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

pub fn is_zst_file(path: &Path) -> std::io::Result<bool> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; 4];

    if reader.read_exact(&mut buffer).is_err() {
        return Ok(false);
    }

    Ok(buffer == [0x28, 0xB5, 0x2F, 0xFD])
}

pub fn is_zst_seekable(path: &Path) -> std::io::Result<bool> {
    let file = File::open(path)?;

    match zstd_seekable::Seekable::init(Box::new(file)) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
