use super::*;

use std::fs::File;

impl SparseFile for File {
    fn scan_chunks(&mut self) -> std::result::Result<std::vec::Vec<Segment>, ScanError> {
        todo!()
    }
}
