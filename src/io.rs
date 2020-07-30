use std::{
    fs::File,
    io::{
        BufReader,
        Read,
        Seek,
        SeekFrom,
    },
    mem,
    path::Path,
};

use crate::{
    deps::log::debug,
    error::Error,
};


pub fn read_u64(rdr: &mut dyn Read) -> Result<u64, Error> {
    let mut buffer = 0u64.to_ne_bytes();
    rdr.read_exact(&mut buffer[..])?;
    Ok(u64::from_ne_bytes(buffer))
}


pub fn new_buffered_file_reader(
    path: &Path,
    offset: Option<std::num::NonZeroU64>,
) -> Result<BufReader<File>, std::io::Error> {
    let mut reader = BufReader::new(open_raw_file(path, offset)?);
    Ok(reader)
}


pub fn open_raw_file(
    path: &Path,
    offset: Option<std::num::NonZeroU64>,
) -> Result<File, std::io::Error> {
    debug!("opening file: {:?}", path);
    let mut reader = std::fs::File::open(&path)?;
    if let Some(start_offset) = offset {
        let seek = SeekFrom::Start(start_offset.get());
        debug!("seek to {} in file: {:?}", start_offset.get(), path);
        reader.seek(seek)?;
    }

    Ok(reader)
}
