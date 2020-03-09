#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
use super::*;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::windows::io::{AsRawHandle, RawHandle};

use winapi::um::fileapi::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::winioctl::FSCTL_QUERY_ALLOCATED_RANGES;
use winapi::um::winnt::FILE_ATTRIBUTE_SPARSE_FILE;

use std::mem::MaybeUninit;

struct Range {
    start: u64,
    end: u64,
}

impl SparseFile for File {
    fn scan_chunks(&mut self) -> std::result::Result<std::vec::Vec<Segment>, ScanError> {
        // get the handle from the file
        let handle = self.as_raw_handle();
        // Check for sparsity
        if is_sparse(handle)? {
            // Call through and get the allocated ranges
            todo!()
        } else {
            let len = self.seek(SeekFrom::End(0))?;
            Ok(vec![Segment {
                segment_type: SegmentType::Data,
                start: 0,
                end: len,
            }])
        }
    }
}

fn get_allocated_ranges(handle: RawHandle) -> Result<Vec<Range>, ScanError> {
    todo!()
}

/// Check if the file is sparse
///
/// This will allow us to skip the nonsense and return a single range if it isn't
fn is_sparse(handle: RawHandle) -> Result<bool, ScanError> {
    // Create a space for the file_info to go
    let mut file_info: MaybeUninit<BY_HANDLE_FILE_INFORMATION> = MaybeUninit::zeroed();
    // Make the call
    let ret = unsafe { GetFileInformationByHandle(handle, file_info.as_mut_ptr()) };
    // Check for an error and indicate if there was one
    if ret == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // Now that we have the file info, unwrap it, we would have returned by now if it was still uninitialized
    let file_info = unsafe { file_info.assume_init() };
    Ok(file_info.dwFileAttributes & FILE_ATTRIBUTE_SPARSE_FILE != 0)
}
