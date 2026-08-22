//! Windows Shell and Context Menu integration via native Win32 Registry APIs.

use std::env;
use std::io;

/// Checks if CopyFile is registered in the Windows Context Menu via native Win32 Registry API.
#[must_use]
pub fn is_context_menu_registered() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        };

        let subkey: Vec<u16> =
            std::ffi::OsStr::new("Software\\Classes\\Directory\\shell\\CopyFile")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

        let mut hkey: HKEY = std::ptr::null_mut();
        let result =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) };

        if result == 0 {
            unsafe { RegCloseKey(hkey) };
            true
        } else {
            false
        }
    }

    #[cfg(not(windows))]
    {
        false
    }
}

/// Registers CopyFile into the current user's Windows Context Menu via native Win32 Registry API.
///
/// # Errors
/// Returns an error if registry operations fail.
pub fn register_context_menu() -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE,
            REG_OPTION_NON_VOLATILE, REG_SZ,
        };

        let exe_path = env::current_exe()?;
        let exe_str = exe_path.to_string_lossy();

        let contexts = [
            (
                "Software\\Classes\\Directory\\shell\\CopyFile",
                "Copiar com CopyFile",
                format!("\"{exe_str}\" -s \"%1\""),
            ),
            (
                "Software\\Classes\\Directory\\Background\\shell\\CopyFile",
                "Copiar com CopyFile",
                format!("\"{exe_str}\" -s \"%V\""),
            ),
            (
                "Software\\Classes\\Drive\\shell\\CopyFile",
                "Copiar com CopyFile",
                format!("\"{exe_str}\" -s \"%1\""),
            ),
            (
                "Software\\Classes\\*\\shell\\CopyFile",
                "Copiar com CopyFile",
                format!("\"{exe_str}\" -s \"%1\""),
            ),
            // Paste with CopyFile in Folder Background, Drives, and Desktop
            (
                "Software\\Classes\\Directory\\Background\\shell\\CopyFilePaste",
                "Colar com CopyFile",
                format!("\"{exe_str}\" --paste-to \"%V\""),
            ),
            (
                "Software\\Classes\\Drive\\Background\\shell\\CopyFilePaste",
                "Colar com CopyFile",
                format!("\"{exe_str}\" --paste-to \"%V\""),
            ),
            (
                "Software\\Classes\\DesktopBackground\\shell\\CopyFilePaste",
                "Colar com CopyFile",
                format!("\"{exe_str}\" --paste-to \"%V\""),
            ),
        ];

        for (base_path, label_str, cmd_str) in contexts {
            let base_wide: Vec<u16> = OsStr::new(base_path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut hkey_base: HKEY = std::ptr::null_mut();
            let res = unsafe {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    base_wide.as_ptr(),
                    0,
                    std::ptr::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_WRITE,
                    std::ptr::null(),
                    &mut hkey_base,
                    std::ptr::null_mut(),
                )
            };

            if res == 0 {
                // Set default value (label)
                let label_wide: Vec<u16> = OsStr::new(label_str)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                unsafe {
                    RegSetValueExW(
                        hkey_base,
                        std::ptr::null(),
                        0,
                        REG_SZ,
                        label_wide.as_ptr() as *const u8,
                        (label_wide.len() * 2) as u32,
                    );
                }

                // Set icon
                let icon_name_wide: Vec<u16> = OsStr::new("Icon")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let icon_val_wide: Vec<u16> = OsStr::new(&format!("\"{exe_str}\""))
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                unsafe {
                    RegSetValueExW(
                        hkey_base,
                        icon_name_wide.as_ptr(),
                        0,
                        REG_SZ,
                        icon_val_wide.as_ptr() as *const u8,
                        (icon_val_wide.len() * 2) as u32,
                    );
                }

                // Create command subkey
                let cmd_key_path = format!("{base_path}\\command");
                let cmd_key_wide: Vec<u16> = OsStr::new(&cmd_key_path)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let mut hkey_cmd: HKEY = std::ptr::null_mut();
                let cmd_res = unsafe {
                    RegCreateKeyExW(
                        HKEY_CURRENT_USER,
                        cmd_key_wide.as_ptr(),
                        0,
                        std::ptr::null(),
                        REG_OPTION_NON_VOLATILE,
                        KEY_WRITE,
                        std::ptr::null(),
                        &mut hkey_cmd,
                        std::ptr::null_mut(),
                    )
                };

                if cmd_res == 0 {
                    let cmd_val_wide: Vec<u16> = OsStr::new(&cmd_str)
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    unsafe {
                        RegSetValueExW(
                            hkey_cmd,
                            std::ptr::null(),
                            0,
                            REG_SZ,
                            cmd_val_wide.as_ptr() as *const u8,
                            (cmd_val_wide.len() * 2) as u32,
                        );
                        RegCloseKey(hkey_cmd);
                    }
                }

                unsafe { RegCloseKey(hkey_base) };
            }
        }

        Ok(())
    }

    #[cfg(not(windows))]
    {
        Ok(())
    }
}

/// Unregisters CopyFile from the Windows Context Menu via native Win32 Registry API.
///
/// # Errors
/// Returns an error if registry operations fail.
pub fn unregister_context_menu() -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegDeleteKeyW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE,
        };

        let contexts = [
            "Software\\Classes\\Directory\\shell\\CopyFile",
            "Software\\Classes\\Directory\\Background\\shell\\CopyFile",
            "Software\\Classes\\Drive\\shell\\CopyFile",
            "Software\\Classes\\*\\shell\\CopyFile",
            "Software\\Classes\\Directory\\Background\\shell\\CopyFilePaste",
            "Software\\Classes\\Drive\\Background\\shell\\CopyFilePaste",
            "Software\\Classes\\DesktopBackground\\shell\\CopyFilePaste",
        ];

        for base_path in contexts {
            let cmd_path = format!("{base_path}\\command");
            let cmd_wide: Vec<u16> = OsStr::new(&cmd_path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let base_wide: Vec<u16> = OsStr::new(base_path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut hkey: HKEY = std::ptr::null_mut();
            if unsafe {
                RegOpenKeyExW(HKEY_CURRENT_USER, std::ptr::null(), 0, KEY_WRITE, &mut hkey)
            } == 0
            {
                unsafe {
                    let _ = RegDeleteKeyW(hkey, cmd_wide.as_ptr());
                    let _ = RegDeleteKeyW(hkey, base_wide.as_ptr());
                    RegCloseKey(hkey);
                }
            }
        }

        Ok(())
    }

    #[cfg(not(windows))]
    {
        Ok(())
    }
}

/// Copies the current executable to `C:\Windows\copyfile.exe` so it's in the system PATH.
///
/// # Errors
/// Returns an error if copying fails (e.g. requires administrator privileges).
pub fn install_to_system_windows() -> io::Result<std::path::PathBuf> {
    let current_exe = env::current_exe()?;
    let target = std::path::PathBuf::from(r"C:\Windows\copyfile.exe");
    std::fs::copy(&current_exe, &target)?;
    Ok(target)
}

/// Checks if copyfile.exe is present in `C:\Windows\`.
#[must_use]
pub fn is_installed_in_windows() -> bool {
    std::path::PathBuf::from(r"C:\Windows\copyfile.exe").exists()
}
