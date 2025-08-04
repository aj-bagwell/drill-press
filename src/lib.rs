use std::io::{Read, Seek};
use std::ops::Range;
use std::slice::Iter;
use thiserror::Error;

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "linux",
                 target_os = "android",
                 target_os = "freebsd",
                 target_os = "macos",
    ))]{
        mod unix;
    } else if #[cfg(windows)] {
        mod windows;
    } else {
        mod default;
    }
}

#[cfg(test)]
mod test_utils;

#[derive(Error, Debug)]
/// Errors returned by [`scan_chunks`](SparseFile::scan_chunks)
pub enum ScanError {
    #[error("IO Error occurred")]
    IO(#[from] std::io::Error),
    #[error("An unknown error occurred interacting with the C API")]
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

impl SegmentType {
    pub fn opposite(&self) -> Self {
        match self {
            SegmentType::Hole => SegmentType::Data,
            SegmentType::Data => SegmentType::Hole,
        }
    }
}

/// Describes the location of a chunk in the file, as well as indicating if it
/// contains data or is a hole
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Marks this segment as either containing a hole, or containing data
    pub segment_type: SegmentType,
    /// the (half-open) range of bytes in the file covered by this segment
    pub range: Range<u64>,
}

/// An iterator over the ranges of a file of a specific [`SegmentType`]
#[derive(Debug, Clone)]
pub struct SegmentIter<'a> {
    segment_type: SegmentType,
    iter: Iter<'a, Segment>,
}

impl<'a> Iterator for SegmentIter<'a> {
    type Item = &'a Range<u64>;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        for segment in self.iter.by_ref() {
            if segment.segment_type == self.segment_type {
                return Some(&segment.range);
            }
        }
        None
    }
}

/// An extention trait to filter segments by Hole or Data segments
pub trait Segments {
    fn data(&self) -> SegmentIter;
    fn holes(&self) -> SegmentIter;
}

impl Segments for Vec<Segment> {
    fn data(&self) -> SegmentIter {
        SegmentIter {
            segment_type: SegmentType::Data,
            iter: self.iter(),
        }
    }
    fn holes(&self) -> SegmentIter {
        SegmentIter {
            segment_type: SegmentType::Hole,
            iter: self.iter(),
        }
    }
}

#[allow(clippy::len_without_is_empty)] // Segments should never be zero length
impl Segment {
    /// Returns true if the provided offset is within the range of bytes this
    /// segment specifies
    pub fn contains(&self, offset: &u64) -> bool {
        self.range.contains(offset)
    }

    /// Returns true if this segment is a Hole
    pub fn is_hole(&self) -> bool {
        self.segment_type == SegmentType::Hole
    }

    /// Returns true if this segment contains data
    pub fn is_data(&self) -> bool {
        self.segment_type == SegmentType::Data
    }

    /// The starting position of this segment
    pub fn start(&self) -> u64 {
        self.range.start
    }

    /// The number of bytes in this segment
    pub fn len(&self) -> u64 {
        self.range.end - self.range.start
    }
}

/// An extention trait for [`File`](std::fs::File) for sparse files
pub trait SparseFile: Read + Seek {
    /// Scans the file to find its logical chunks
    ///
    /// Will return a list of segments, ordered by their start position.
    ///
    /// The ranges generated are guaranteed to cover all bytes in the file.
    ///
    /// `Hole` segments are guaranteed to represent a part of a file that does
    /// not contain any non-zero data, however, `Data` segments may represent
    /// parts of a file that contain what, logically, should be sparse segments.
    /// This is up to the mercy of your operating system and file system, please
    /// consult their documentation for how they handle sparse files for more
    /// details.
    ///
    /// Does not make any guarantee about maintaining the Seek position of the
    /// file, always seek back to a known point after calling this method.
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

    /// Unallocate a section of the file, freeing the disk space and making
    /// future reads return zeros
    fn drill_hole(&self, start: u64, end: u64) -> Result<(), ScanError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use quickcheck_macros::quickcheck;
    use std::fs::File;

    fn test_chunks_match(file: &mut File, input_segments: &[Segment]) -> bool {
        // Get both sets of segments
        let output_segments = file.scan_chunks().expect("Unable to scan chunks");

        if *input_segments != output_segments {
            println!("Expected: \n {:?} \n", input_segments);
            println!("Got: \n {:?} \n", output_segments);
        }
        *input_segments == output_segments
    }

    // Creates a file based on desc, then tests that the resulting output of
    // file.scan_chunks() matches the description used to create the file
    fn test_round_trips(desc: SparseDescription) -> bool {
        let mut file = desc.to_file();
        // Get both sets of segments
        let input_segments = desc.segments();
        println!("Found {} segments of {} bytes", input_segments.len(), input_segments.clone().into_iter().map(|x| x.len()).sum::<u64>());
        test_chunks_match(file.as_file_mut(), &input_segments)
    }

    #[quickcheck]
    fn round_trips(desc: SparseDescription) -> bool {
        test_round_trips(desc)
    }

    #[quickcheck]
    fn drill_hole(desc: SparseDescription, drop: u8) -> bool {
        let mut file = desc.to_file();
        // Get both sets of segments
        let mut input_segments = desc.segments();

        if input_segments.is_empty() {
            return true;
        }

        #[cfg(target_os = "macos")]
        for hole in input_segments.holes() {
            file.as_file_mut()
                .drill_hole(hole.start, hole.end)
                .expect("pre drill holes");
        }

        test_chunks_match(file.as_file_mut(), &input_segments);

        // pick a segment to make a hole
        let drop_idx = drop as usize % input_segments.len();
        let drop = &mut input_segments[drop_idx];

        file.as_file_mut()
            .drill_hole(drop.range.start, drop.range.end)
            .expect("drilled hole");

        drop.segment_type = SegmentType::Hole;

        combine_segments(&mut input_segments);

        test_chunks_match(file.as_file_mut(), &input_segments)
    }

    #[quickcheck]
    fn one_big_segment(segment_type: SegmentType) -> bool {
        let desc = SparseDescription::one_segment(segment_type, 3545868);

        test_round_trips(desc)
    }

    fn combine_segments(segments: &mut Vec<Segment>) {
        let mut prev = 0;
        for i in 1..segments.len() {
            if segments[prev].segment_type == segments[i].segment_type {
                segments[prev].range.end = segments[i].range.end;
            } else {
                prev += 1;
                segments[prev] = segments[i].clone();
            }
        }
        segments.truncate(prev + 1)
    }
}
