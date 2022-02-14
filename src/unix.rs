//! lseek based implementation that uses `SEEK_DATA` and `SEEK_HOLE` to
//! reconstruct which segments of the file are data or holes
use super::*;

use std::fs::File;
use std::io::Error;
use std::os::unix::io::AsRawFd;

use errno::errno;
use libc::{c_int, lseek, off_t, EINVAL, ENXIO, SEEK_DATA, SEEK_END, SEEK_HOLE};

impl SparseFile for File {
    fn scan_chunks(&mut self) -> std::result::Result<std::vec::Vec<Segment>, ScanError> {
        // Create our output vec
        let mut tags: Vec<Segment> = Vec::new();
        // Extract the raw fd from the file
        let fd = self.as_raw_fd();
        // Find the end
        let end = safe_lseek(fd, 0, SEEK_END)?.unwrap_or(0);

        if end == 0 {
            return Ok(vec![]);
        }

        // Our seeking loop assumes that we know what type the previous segment
        // is, so grab the first hole and if it does not exist or is not at the
        // start add then the file starts with a data block.
        let mut last_seek = safe_lseek(fd, 0, SEEK_HOLE)?.unwrap_or(end);
        let mut last_type = SegmentType::Hole;
        if last_seek > 0 {
            tags.push(Segment {
                segment_type: SegmentType::Data,
                range: 0..last_seek,
            })
        }

        while last_seek < end {
            let seek_type = match last_type {
                SegmentType::Hole => SEEK_DATA,
                SegmentType::Data => SEEK_HOLE,
            };

            let next_seek = safe_lseek(fd, last_seek, seek_type)?.unwrap_or(end);
            tags.push(Segment {
                segment_type: last_type,
                range: last_seek..next_seek,
            });
            last_seek = next_seek;
            last_type = last_type.opposite();
        }
        Ok(tags)
    }
}

fn safe_lseek(fd: c_int, offset: u64, seek_type: c_int) -> Result<Option<u64>, ScanError> {
    unsafe {
        let new_offset = lseek(fd, offset as off_t, seek_type);
        // if the return value of lseek is less than 0, an error has occurred
        if new_offset < 0 {
            // find and deref errno, honestly the scariest thing we do here
            let errno = errno().into();
            match errno {
                // EINVAL indicates that the file system does not support
                // SEEK_HOLE or SEEK_DATA, so we indicate as such
                EINVAL => Err(ScanError::UnsupportedFileSystem),
                // ENXIO indicates that the the file offset we are looking for
                // either doesn't exist, or would be beyond the end of the file.
                // In our case, this just means there is no next segment, so we
                // return Ok(none) to indicate as such.
                ENXIO => Ok(None),
                // None of the other error codes require special handling, so we
                // just turn them into an std::io::Error for user friendliness
                _ => Err(Error::last_os_error().into()),
            }
        } else {
            Ok(Some(new_offset as u64))
        }
    }
}
