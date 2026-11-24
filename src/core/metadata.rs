//! File metadata extraction and preservation (timestamps, attributes, permissions).

use std::fs::{self, File, Metadata};
use std::io;
use std::path::Path;

#[cfg(not(windows))]
use filetime::{set_file_times, FileTime};

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

        s.push(if (attrs & FILE_ATTRIBUTE_READONLY) != 0 {
            'r'
        } else {
            '-'
        });
        s.push(if (attrs & FILE_ATTRIBUTE_HIDDEN) != 0 {
            'h'
        } else {
            '-'
        });
        s.push(if (attrs & FILE_ATTRIBUTE_SYSTEM) != 0 {
            's'
        } else {
            '-'
        });
        s.push(if (attrs & FILE_ATTRIBUTE_ARCHIVE) != 0 {
            'a'
        } else {
            '-'
        });

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

/// Applies creation, access, and write timestamps directly to an already-open destination file handle.
/// Avoids an extra `CreateFileW` + `CloseHandle` syscall per copied file on Windows.
#[cfg(windows)]
pub fn apply_windows_times_to_handle(dst_file: &File, src_meta: &Metadata) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{FILETIME, HANDLE};
    use windows_sys::Win32::Storage::FileSystem::SetFileTime;

    let creation_time = src_meta.creation_time();
    let last_access_time = src_meta.last_access_time();
    let last_write_time = src_meta.last_write_time();

    let ft_create = FILETIME {
        dwLowDateTime: creation_time as u32,
        dwHighDateTime: (creation_time >> 32) as u32,
    };
    let ft_access = FILETIME {
        dwLowDateTime: last_access_time as u32,
        dwHighDateTime: (last_access_time >> 32) as u32,
    };
    let ft_write = FILETIME {
        dwLowDateTime: last_write_time as u32,
        dwHighDateTime: (last_write_time >> 32) as u32,
    };

    unsafe {
        let handle = dst_file.as_raw_handle() as HANDLE;
        SetFileTime(handle, &ft_create, &ft_access, &ft_write);
    }
}

/// Applies Windows file attributes (ReadOnly, Hidden, System, Archive) to `dst_path`.
#[cfg(windows)]
pub fn apply_windows_attributes(dst_path: &Path, src_meta: &Metadata) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        SetFileAttributesW, FILE_ATTRIBUTE_ARCHIVE, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY,
        FILE_ATTRIBUTE_SYSTEM,
    };

    let mask = FILE_ATTRIBUTE_READONLY
        | FILE_ATTRIBUTE_HIDDEN
        | FILE_ATTRIBUTE_SYSTEM
        | FILE_ATTRIBUTE_ARCHIVE;
    let attrs = src_meta.file_attributes() & mask;
    // Skip syscall when file only has the default ARCHIVE attribute already set on newly created files
    if attrs == 0 || attrs == FILE_ATTRIBUTE_ARCHIVE {
        return;
    }

    let wide_dst: Vec<u16> = dst_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        SetFileAttributesW(wide_dst.as_ptr(), attrs);
    }
}

/// Clears the Windows read-only attribute on a file so it can be overwritten cleanly.
pub fn clear_readonly_if_needed(path: &Path) {
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = fs::set_permissions(path, perms);
        }
    }
}

/// Preserves timestamps and attributes from `src_path` to `dst_path`.
///
/// On Windows, sets creation/access/write timestamps BEFORE applying Read-Only attributes
/// in a single handle open, fixing read-only timestamp preservation and removing redundant syscalls.
///
/// # Errors
/// Returns an `io::Error` if reading source metadata fails.
pub fn copy_metadata(src_path: &Path, dst_path: &Path) -> io::Result<()> {
    let src_meta = fs::metadata(src_path)?;

    #[cfg(windows)]
    {
        copy_windows_metadata(dst_path, &src_meta)?;
    }

    #[cfg(not(windows))]
    {
        let atime = FileTime::from_last_access_time(&src_meta);
        let mtime = FileTime::from_last_modification_time(&src_meta);
        let _ = set_file_times(dst_path, atime, mtime);

        let permissions = src_meta.permissions();
        let _ = fs::set_permissions(dst_path, permissions);
    }

    Ok(())
}

#[cfg(windows)]
fn copy_windows_metadata(dst_path: &Path, src_meta: &Metadata) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, SetFileAttributesW, SetFileTime, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_READ,
        FILE_SHARE_WRITE, FILE_WRITE_ATTRIBUTES, OPEN_EXISTING,
    };

    let wide_dst: Vec<u16> = dst_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let creation_time = src_meta.creation_time();
    let last_access_time = src_meta.last_access_time();
    let last_write_time = src_meta.last_write_time();

    let ft_create = FILETIME {
        dwLowDateTime: creation_time as u32,
        dwHighDateTime: (creation_time >> 32) as u32,
    };
    let ft_access = FILETIME {
        dwLowDateTime: last_access_time as u32,
        dwHighDateTime: (last_access_time >> 32) as u32,
    };
    let ft_write = FILETIME {
        dwLowDateTime: last_write_time as u32,
        dwHighDateTime: (last_write_time >> 32) as u32,
    };

    unsafe {
        // 1. Set timestamps FIRST (before ReadOnly attribute might block write-attributes)
        let handle: HANDLE = CreateFileW(
            wide_dst.as_ptr(),
            FILE_WRITE_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        );

        if handle != INVALID_HANDLE_VALUE {
            SetFileTime(handle, &ft_create, &ft_access, &ft_write);
            CloseHandle(handle);
        }

        // 2. Apply Windows file attributes (ReadOnly, Hidden, System, Archive)
        let attrs = src_meta.file_attributes();
        if attrs != 0 {
            SetFileAttributesW(wide_dst.as_ptr(), attrs);
        }
    }

    Ok(())
}
