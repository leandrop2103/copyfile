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
        let label_str = "Copiar com CopyFile";

        let contexts = [
            (
                "Software\\Classes\\Directory\\shell\\CopyFile",
                format!("\"{exe_str}\" -s \"%1\""),
            ),
            (
                "Software\\Classes\\Directory\\Background\\shell\\CopyFile",
                format!("\"{exe_str}\" -s \"%V\""),
            ),
            (
                "Software\\Classes\\Drive\\shell\\CopyFile",
                format!("\"{exe_str}\" -s \"%1\""),
            ),
            (
                "Software\\Classes\\*\\shell\\CopyFile",
                format!("\"{exe_str}\" -s \"%1\""),
            ),
        ];

        for (base_path, cmd_str) in contexts {
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

                // Set icon (displays the CopyFile icon right beside the context menu item in Explorer)
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

                let res_cmd = unsafe {
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

                if res_cmd == 0 {
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
/// Returns an error if removing registry keys fails.
pub fn unregister_context_menu() -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Registry::{RegDeleteTreeW, HKEY_CURRENT_USER};

        let paths_to_delete = [
            "Software\\Classes\\Directory\\shell\\CopyFile",
            "Software\\Classes\\Directory\\Background\\shell\\CopyFile",
            "Software\\Classes\\Drive\\shell\\CopyFile",
            "Software\\Classes\\*\\shell\\CopyFile",
        ];

        for base_path in paths_to_delete {
            let wide: Vec<u16> = OsStr::new(base_path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                RegDeleteTreeW(HKEY_CURRENT_USER, wide.as_ptr());
            }
        }

        Ok(())
    }

    #[cfg(not(windows))]
    {
        Ok(())
    }
}
