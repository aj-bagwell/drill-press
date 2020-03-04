//! lseek based implemenation that uses `SEEK_DATA` and `SEEK_HOLE` to
//! reconstruct which segements of the file are data or holes
use super::*;

use std::fs::File;

impl SparseFile for File {
    fn scan_chunks(&mut self) -> std::result::Result<std::vec::Vec<Segment>, ScanError> {
        todo!()
    }
}
