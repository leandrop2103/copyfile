//! Windows Clipboard and Explorer active path inspection.

use std::path::PathBuf;

/// Reads the list of files currently present in the Windows Clipboard (`CF_HDROP`).
#[must_use]
pub fn get_clipboard_files() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::System::DataExchange::{
            CloseClipboard, GetClipboardData, OpenClipboard,
        };
        use windows_sys::Win32::UI::Shell::DragQueryFileW;

        const CF_HDROP: u32 = 15;

        let mut files = Vec::new();

        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return files;
            }

            let h_drop = GetClipboardData(CF_HDROP);
            if !h_drop.is_null() {
                let count = DragQueryFileW(h_drop as _, 0xFFFF_FFFF, std::ptr::null_mut(), 0);
                for i in 0..count {
                    let len = DragQueryFileW(h_drop as _, i, std::ptr::null_mut(), 0);
                    if len > 0 {
                        let mut buffer: Vec<u16> = vec![0; (len + 1) as usize];
                        DragQueryFileW(h_drop as _, i, buffer.as_mut_ptr(), len + 1);
                        buffer.truncate(len as usize);
                        let os_str = OsString::from_wide(&buffer);
                        files.push(PathBuf::from(os_str));
                    }
                }
            }

            CloseClipboard();
        }

        files
    }

    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Checks whether the Windows Clipboard currently holds any file/folder drops.
#[must_use]
pub fn has_clipboard_files() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::DataExchange::IsClipboardFormatAvailable;
        const CF_HDROP: u32 = 15;
        unsafe { IsClipboardFormatAvailable(CF_HDROP) != 0 }
    }

    #[cfg(not(windows))]
    {
        false
    }
}

/// Retrieves the active folder path of the foreground Windows Explorer or Desktop window via PowerShell / COM.
#[must_use]
pub fn get_active_explorer_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use std::process::Command;

        // Query active explorer or desktop folder via fast powershell snippet
        let script = r#"
            $shell = New-Object -ComObject Shell.Application
            $activeWindow = (Get-Process | Where-Object { $_.MainWindowHandle -eq (Get-ForegroundWindow) } | Select-Object -First 1).Id
            foreach ($w in $shell.Windows()) {
                try {
                    $path = $w.Document.Folder.Self.Path
                    if ($path -and (Test-Path $path)) {
                        Write-Output $path
                        exit 0
                    }
                } catch {}
            }
            # Fallback to desktop if desktop is active
            [Environment]::GetFolderPath('Desktop')
        "#;

        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .ok()?;

        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            let path = PathBuf::from(path_str);
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    #[cfg(not(windows))]
    {
        None
    }
}
