/*!
Utility methods for locating holes in sparse files
*/

use std::io::{Read, Seek};
use thiserror::Error;

cfg_if::cfg_if! {
    if #[cfg(unix)]{
        mod unix;
    } else {
        mod default;
    }
}

#[cfg(test)]
mod test_utils;

#[derive(Error, Debug)]
pub enum ScanError {
    #[error("IO Error occured")]
    IO(#[from] std::io::Error),
    #[error("An unkown error occured interating with the C API")]
    Raw(i32),
    #[error("The operation you are trying to perform is not supported on this platform")]
    UnsupportedPlatform,
    #[error("The filesystem does not support operating on sparse files")]
    UnsupportedFileSystem,
}

/// Flag for determining if a segment is a hole, or if it contains data
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SegmentType {
    Hole,
    Data,
}

/// Describes the location of a chunk in the file, as well as indicating if it
/// contains data or is a hole
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    /// Will return `Err(ScanError::UnsupportedFileSystem)` if support is
    /// implemented for your operating system, but the filesystem does not
    /// support sparse files
    ///
    /// Will also return `Err` if any other I/O error occurs
    fn scan_chunks(&mut self) -> Result<Vec<Segment>, ScanError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use quickcheck_macros::quickcheck;

    fn test_covers_all_bytes(desc: SparseDescription) -> bool {
        let mut file = desc.to_file();
        // Get both sets of segments
        let input_segments = desc.segments();
        let output_segments = file.scan_chunks().expect("Unable to scan chunks");
        println!("Ouput: \n {:?} \n", output_segments);
        // Find the last non-zero byte in the input segments
        let last_non_zero = input_segments
            .iter()
            .map(|x| {
                if let SegmentType::Data = x.segment_type {
                    x.end
                } else {
                    0
                }
            })
            .max()
            .unwrap_or(0);
        println!("Last non zero: {} \n", last_non_zero);
        let mut last_byte_touched = false;
        for (x, y) in output_segments.iter().zip(output_segments.iter().skip(1)) {
            if y.start != x.end + 1 {
                return false;
            }
            if y.end >= last_non_zero {
                println!("Last byte touched!");
                last_byte_touched = true;
            }
        }
        if output_segments.len() == 1 {
            if output_segments[0].end >= last_non_zero {
                println!("Last byte touched!");
                last_byte_touched = true;
            }
        }
        last_byte_touched || last_non_zero == 0
    }

    #[quickcheck]
    fn covers_all_bytes(desc: SparseDescription) -> bool {
        test_covers_all_bytes(desc)
    }

    #[test]
    fn covers_all_bytes_failure_1() {
        let desc = SparseDescription::from_segments(vec![
            Segment {
                segment_type: SegmentType::Hole,
                start: 0,
                end: 3545867,
            },
            Segment {
                segment_type: SegmentType::Data,
                start: 3545868,
                end: 3625675,
            },
        ]);

        assert!(test_covers_all_bytes(desc));
    }

    #[test]
    fn covers_all_bytes_failure_2() {
        let desc = SparseDescription::from_segments(vec![Segment {
            segment_type: SegmentType::Hole,
            start: 0,
            end: 5440262,
        }]);

        assert!(test_covers_all_bytes(desc));
    }
}
