use std::io::{Read, Seek};
use thiserror::Error;

cfg_if::cfg_if! {
    if #[cfg(unix)]{
        mod unix;
    } else {
        mod default;
    }
}
#[derive(Error, Debug)]
pub enum ScanError {
    #[error("IO Error occured")]
    IO(#[from] std::io::Error),
    #[error("An unkown error occured interating with the C API")]
    Raw(i32),
    #[error("The operation you are trying to perform is not supported on this platform")]
    UnsupportedPlatform,
}

/// Flag for determining if a segment is a hole, or if it contains data
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SegmentType {
    Hole,
    Data,
}

/// Describes the location of a chunk in the file, as well as indicating if it
/// contains data or is a hole
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Marks this segment as either contianing a hole, or containing data
    pub segment_type: SegmentType,
    pub start: u64,
    pub end: u64,
}

/// Trait for objects that can have sparisty
pub trait SparseFile: Read + Seek {
    /// Scans the file to find its logical chunks
    ///
    /// Will return a list of segments, ordered by their start position.
    ///
    /// The ranges generated are guarenteed to cover all bytes in the file, up
    /// to the last non-zero byte in the last segement containing data. All
    /// files are considered to have a single hole of indeterminate length at
    /// the end, and this library will not included that hole.
    ///
    /// `Hole` segements are guarenteed to represent a part of a file that does
    /// not contain any non-zero data, however, `Data` segements may represent
    /// parts of a file that contain what, logically, should be sparse segments.
    /// This is up to the mercy of your operating system and file system, please
    /// consult their documentation for how they handle sparse files for more
    /// details.
    ///
    /// Does not make any guarantee about mainting the Seek pointer of the file,
    /// always seek back to a known point after calling this method.
    ///
    /// # Errors
    ///
    /// Will return `Err(ScanError::UnsupportedPlatform)` if support is not
    /// implemented for filesystem level hole finding on your system
    ///
    /// Will also return `Err` if any other I/O error occurs
    fn scan_chunks(&mut self) -> Result<Vec<Segment>, ScanError>;
}
