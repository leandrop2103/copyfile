//! File metadata extraction and preservation (timestamps, attributes, permissions).

use filetime::{set_file_times, FileTime};
use std::fs::{self, Metadata};
use std::io;
use std::path::Path;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

/// Extracts a concise attributes string (like `-a--`, `rhsa`, etc.).
#[must_use]
pub fn extract_attributes_string(_path: &Path, metadata: &Metadata) -> String {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY,
            FILE_ATTRIBUTE_SYSTEM,
        };

        let attrs = metadata.file_attributes();
        let mut s = String::with_capacity(4);

        if (attrs & FILE_ATTRIBUTE_READONLY) != 0 {
            s.push('r');
        } else {
            s.push('-');
        }

        if (attrs & FILE_ATTRIBUTE_HIDDEN) != 0 {
            s.push('h');
        } else {
            s.push('-');
        }

        if (attrs & FILE_ATTRIBUTE_SYSTEM) != 0 {
            s.push('s');
        } else {
            s.push('-');
        }

        if (attrs & FILE_ATTRIBUTE_ARCHIVE) != 0 {
            s.push('a');
        } else {
            s.push('-');
        }

        s
    }

    #[cfg(not(windows))]
    {
        let readonly = metadata.permissions().readonly();
        if readonly {
            "r---".to_string()
        } else {
            "-a--".to_string()
        }
    }
}

/// Preserves timestamps and attributes from `src_path` to `dst_path`.
///
/// # Errors
/// Returns an `io::Error` if updating timestamps or attributes fails.
pub fn copy_metadata(src_path: &Path, dst_path: &Path) -> io::Result<()> {
    let src_meta = fs::metadata(src_path)?;

    // 1. Copy permissions (read-only flag / Unix mode)
    let permissions = src_meta.permissions();
    let _ = fs::set_permissions(dst_path, permissions);

    // 2. Copy Timestamps (Access Time & Modification Time)
    let atime = FileTime::from_last_access_time(&src_meta);
    let mtime = FileTime::from_last_modification_time(&src_meta);
    let _ = set_file_times(dst_path, atime, mtime);

    // 3. Platform specific: Windows creation time and file attributes
    #[cfg(windows)]
    {
        copy_windows_metadata(src_path, dst_path, &src_meta)?;
    }

    Ok(())
}

#[cfg(windows)]
fn copy_windows_metadata(_src_path: &Path, dst_path: &Path, src_meta: &Metadata) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, SetFileAttributesW, SetFileTime, FILE_SHARE_READ, FILE_SHARE_WRITE,
        FILE_WRITE_ATTRIBUTES, OPEN_EXISTING,
    };

    // Set Windows Attributes (ReadOnly, Hidden, System, Archive)
    let attrs = src_meta.file_attributes();
    let wide_dst: Vec<u16> = dst_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // Preserve Windows Creation Time (Birthtime)
    let creation_time = src_meta.creation_time();
    let last_access_time = src_meta.last_access_time();
    let last_write_time = src_meta.last_write_time();

    let ft_create = windows_sys::Win32::Foundation::FILETIME {
        dwLowDateTime: creation_time as u32,
        dwHighDateTime: (creation_time >> 32) as u32,
    };
    let ft_access = windows_sys::Win32::Foundation::FILETIME {
        dwLowDateTime: last_access_time as u32,
        dwHighDateTime: (last_access_time >> 32) as u32,
    };
    let ft_write = windows_sys::Win32::Foundation::FILETIME {
        dwLowDateTime: last_write_time as u32,
        dwHighDateTime: (last_write_time >> 32) as u32,
    };

    unsafe {
        // Open handle with write attributes permission
        let handle: HANDLE = CreateFileW(
            wide_dst.as_ptr(),
            FILE_WRITE_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        );

        if handle != INVALID_HANDLE_VALUE {
            SetFileTime(handle, &ft_create, &ft_access, &ft_write);
            CloseHandle(handle);
        }

        // Apply file attributes (after setting times, in case read-only was set)
        if attrs != 0 {
            SetFileAttributesW(wide_dst.as_ptr(), attrs);
        }
    }

    Ok(())
}
