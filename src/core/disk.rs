//! Disk space utilities for checking destination capacity before and during copy.

use std::path::Path;

/// Information about disk storage capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskSpaceInfo {
    /// Bytes available to the current user.
    pub available_bytes: u64,
    /// Total capacity of the disk volume in bytes.
    pub total_bytes: u64,
}

/// Retrieves available and total disk space for the given filesystem path.
#[must_use]
pub fn get_disk_space(path: &Path) -> Option<DiskSpaceInfo> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

        let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide_path.push(0);

        let mut free_bytes_available: u64 = 0;
        let mut total_number_of_bytes: u64 = 0;
        let mut total_number_of_free_bytes: u64 = 0;

        let success = unsafe {
            GetDiskFreeSpaceExW(
                wide_path.as_ptr(),
                &mut free_bytes_available as *mut u64 as *mut _,
                &mut total_number_of_bytes as *mut u64 as *mut _,
                &mut total_number_of_free_bytes as *mut u64 as *mut _,
            )
        };

        if success != 0 {
            Some(DiskSpaceInfo {
                available_bytes: free_bytes_available,
                total_bytes: total_number_of_bytes,
            })
        } else {
            // If path doesn't exist yet, try its parent
            if let Some(parent) = path.parent() {
                return get_disk_space(parent);
            }
            None
        }
    }

    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}
