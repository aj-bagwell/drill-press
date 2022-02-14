use super::*;

use std::io::{Seek, SeekFrom, Write};
use tempfile::NamedTempFile;

use quickcheck::{Arbitrary, Gen};

const BLOCK_SIZE: u64 = 4 * 1024;
const MAX_SPLITS: usize = 50;

#[derive(Clone, Debug)]
pub struct SparseDescription {
    start_type: SegmentType,
    split_points: Vec<u8>,
}

impl SparseDescription {
    pub fn from_parts(start_type: SegmentType, mut split_points: Vec<u8>) -> Self {
        split_points.retain(|x| *x != 0);
        split_points.truncate(MAX_SPLITS);
        split_points.sort_unstable();
        split_points.dedup();

        SparseDescription {
            start_type,
            split_points,
        }
    }

    pub fn segments(&self) -> Vec<Segment> {
        let mut segment_type = self.start_type;

        let mut prev = 0;

        let mut segments = Vec::with_capacity(self.split_points.len() + 1);

        for point in &self.split_points {
            let point = *point as u64 * BLOCK_SIZE;

            segments.push(Segment {
                segment_type,
                range: prev..point,
            });
            prev = point;
            segment_type = segment_type.opposite();
        }

        segments
    }

    pub fn one_segment(start_type: SegmentType, end: u64) -> Self {
        SparseDescription::from_parts(start_type, vec![(end / BLOCK_SIZE) as u8])
    }

    pub fn to_file(&self) -> NamedTempFile {
        let mut temp = NamedTempFile::new().expect("Unable to create tempfile");

        // Special handling to enable sparsity on windows
        #[cfg(windows)]
        {
            use std::process::Command;
            Command::new("fsutil")
                .arg("sparse")
                .arg("setflag")
                .arg(temp.path())
                .output()
                .expect("Unable to set the sparse flag on the tempfile");
        }

        let file = temp.as_file_mut();
        // Iterate through the SparseDescription
        for segment in self.segments().data() {
            file.seek(SeekFrom::Start(segment.start))
                .expect("Unable to seek in file");
            let length = segment.end - segment.start;
            let buffer = vec![1_u8; length as usize];
            file.write_all(&buffer[..])
                .expect("Unable to write bytes to file");
        }

        let last = self.split_points.last().copied().unwrap_or_default();
        temp.as_file_mut()
            .set_len(last as u64 * BLOCK_SIZE)
            .expect("Unable to set length of file");
        temp
    }
}

impl Arbitrary for SparseDescription {
    fn arbitrary(g: &mut Gen) -> Self {
        // Generate some random points in the file to be boundarires between segments
        SparseDescription::from_parts(SegmentType::arbitrary(g), Vec::arbitrary(g))
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let start_type = self.start_type;
        Box::new(
            self.split_points
                .shrink()
                .map(move |split_points| SparseDescription::from_parts(start_type, split_points)),
        )
    }
}

impl Arbitrary for SegmentType {
    fn arbitrary(g: &mut Gen) -> Self {
        if bool::arbitrary(g) {
            SegmentType::Hole
        } else {
            SegmentType::Data
        }
    }
}
