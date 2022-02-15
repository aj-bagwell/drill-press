use super::*;

use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::os::windows::io::{AsRawHandle, RawHandle};

use winapi::shared::minwindef::{DWORD, LPVOID};
use winapi::um::fileapi::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::winioctl::FSCTL_QUERY_ALLOCATED_RANGES;
use winapi::um::winnt::FILE_ATTRIBUTE_SPARSE_FILE;

use std::mem::MaybeUninit;

impl SparseFile for File {
    fn scan_chunks(&mut self) -> std::result::Result<std::vec::Vec<Segment>, ScanError> {
        // Get the length before doing anything
        let len = self.seek(SeekFrom::End(0))?;
        // get the handle from the file
        let handle = self.as_raw_handle();
        // First check for an empty file
        if len == 0 {
            // Return nothing here, an empty file has no ranges
            Ok(vec![])
        } else if is_sparse(handle)? {
            // Call through and get the allocated ranges
            let ranges = get_allocated_ranges(handle, len)?;
            // Make a place to put our segments, and copy over our ranges

            let mut prev_end = 0;
            let mut segments = Vec::with_capacity(ranges.len() * 2 + 1);

            for range in ranges {
                let end = range.offset + range.length;
                if prev_end != range.offset {
                    segments.push(Segment {
                        segment_type: SegmentType::Hole,
                        range: prev_end..range.offset,
                    });
                }
                segments.push(Segment {
                    segment_type: SegmentType::Data,
                    range: range.offset..end,
                });
                prev_end = end;
            }

            // Check to see if we need to add a hole segment at the end
            if prev_end < len {
                segments.push(Segment {
                    segment_type: SegmentType::Hole,
                    range: prev_end..len,
                });
            }

            Ok(segments)
        } else {
            Ok(vec![Segment {
                segment_type: SegmentType::Data,
                range: 0..len,
            }])
        }
    }
}

// Define some types
#[repr(C)]
#[derive(Clone, Copy)]
struct FileAllocatedRange {
    offset: u64,
    length: u64,
}

/// Get the portions of a file that contain data
fn get_allocated_ranges(
    handle: RawHandle,
    size: u64,
) -> Result<Vec<FileAllocatedRange>, ScanError> {
    const LEN: usize = 1024;

    let mut query_range_buffer = FileAllocatedRange {
        offset: 0,
        length: size,
    };

    let mut ranges = Vec::with_capacity(LEN);
    let mut returned_bytes: DWORD = 0;

    unsafe {
        let ret = DeviceIoControl(
            handle,
            FSCTL_QUERY_ALLOCATED_RANGES,
            &mut query_range_buffer as *mut _ as LPVOID,
            std::mem::size_of::<FileAllocatedRange>() as DWORD,
            ranges.as_mut_ptr() as LPVOID,
            (ranges.capacity() * std::mem::size_of::<FileAllocatedRange>()) as DWORD,
            &mut returned_bytes,
            std::ptr::null_mut(),
        );
        // Check the returned value
        // FIXME: WIll error if the user provides a massive file with too many ranges
        // Really need to check for MORE_DATA and do a loop
        if ret == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        ranges.set_len(returned_bytes as usize / std::mem::size_of::<FileAllocatedRange>());
    };

    Ok(ranges)
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
