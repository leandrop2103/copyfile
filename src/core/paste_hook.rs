//! Windows Explorer Ctrl+V keyboard interceptor daemon and background listener.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

static HOOK_RUNNING: AtomicBool = AtomicBool::new(false);

/// Starts the background thread that monitors `Ctrl+V` within Windows Explorer and Desktop.
pub fn start_paste_interceptor(is_enabled: Arc<AtomicBool>) {
    if HOOK_RUNNING.swap(true, Ordering::SeqCst) {
        return; // Already running
    }

    thread::spawn(move || {
        #[cfg(windows)]
        {
            use crate::core::clipboard::{get_active_explorer_path, has_clipboard_files};
            use std::process::Command;
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow};

            const VK_V: i32 = 0x56;

            let mut was_pressed = false;

            loop {
                thread::sleep(Duration::from_millis(50));

                if !is_enabled.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }

                let ctrl_down =
                    unsafe { (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0 };
                let v_down = unsafe { (GetAsyncKeyState(VK_V) as u16 & 0x8000) != 0 };

                if ctrl_down && v_down {
                    if !was_pressed {
                        was_pressed = true;

                        // Check if active foreground window is Windows Explorer or Desktop
                        let hwnd = unsafe { GetForegroundWindow() };
                        if !hwnd.is_null() {
                            let mut class_buf = [0u16; 256];
                            let len = unsafe {
                                GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32)
                            };

                            if len > 0 {
                                let class_name =
                                    String::from_utf16_lossy(&class_buf[..len as usize]);
                                let is_explorer = class_name == "CabinetWClass"
                                    || class_name == "ExploreWClass"
                                    || class_name == "Progman"
                                    || class_name == "WorkerW";

                                if is_explorer && has_clipboard_files() {
                                    // Detect active folder path
                                    let dest_folder = get_active_explorer_path()
                                        .unwrap_or_else(|| PathBuf::from("."));

                                    let exe_path = if crate::core::shell::is_installed_in_windows()
                                    {
                                        PathBuf::from(r"C:\Windows\copyfile.exe")
                                    } else {
                                        std::env::current_exe()
                                            .unwrap_or_else(|_| PathBuf::from("copyfile.exe"))
                                    };

                                    let _ = Command::new(exe_path)
                                        .arg("--prompt-paste")
                                        .arg(dest_folder)
                                        .spawn();

                                    // Debounce to prevent multiple triggers
                                    thread::sleep(Duration::from_millis(600));
                                }
                            }
                        }
                    }
                } else {
                    was_pressed = false;
                }
            }
        }
    });
}

/// Checks whether CopyFile is configured to run at Windows Startup.
#[must_use]
pub fn is_autostart_registered() -> bool {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        };

        let subkey: Vec<u16> = OsStr::new("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut hkey: HKEY = std::ptr::null_mut();
        let res =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) };

        if res == 0 {
            use windows_sys::Win32::System::Registry::RegQueryValueExW;
            let val_name: Vec<u16> = OsStr::new("CopyFileWatcher")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let mut val_type = 0;
            let mut val_size = 0;
            let exists = unsafe {
                RegQueryValueExW(
                    hkey,
                    val_name.as_ptr(),
                    std::ptr::null_mut(),
                    &mut val_type,
                    std::ptr::null_mut(),
                    &mut val_size,
                ) == 0
            };
            unsafe { RegCloseKey(hkey) };
            exists
        } else {
            false
        }
    }

    #[cfg(not(windows))]
    {
        false
    }
}

/// Registers or unregisters CopyFile to run in background at Windows Startup.
///
/// # Errors
/// Returns an error if registry operations fail.
pub fn set_autostart_registered(enable: bool) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
            KEY_WRITE, REG_SZ,
        };

        let subkey: Vec<u16> = OsStr::new("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut hkey: HKEY = std::ptr::null_mut();
        let res =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_WRITE, &mut hkey) };

        if res == 0 {
            let val_name: Vec<u16> = OsStr::new("CopyFileWatcher")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            if enable {
                let exe_path = if crate::core::shell::is_installed_in_windows() {
                    PathBuf::from(r"C:\Windows\copyfile.exe")
                } else {
                    std::env::current_exe()?
                };
                let cmd_str = format!("\"{}\" --watch-paste", exe_path.to_string_lossy());
                let cmd_wide: Vec<u16> = OsStr::new(&cmd_str)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                unsafe {
                    RegSetValueExW(
                        hkey,
                        val_name.as_ptr(),
                        0,
                        REG_SZ,
                        cmd_wide.as_ptr() as *const u8,
                        (cmd_wide.len() * 2) as u32,
                    );
                }
            } else {
                unsafe {
                    let _ = RegDeleteValueW(hkey, val_name.as_ptr());
                }
            }

            unsafe { RegCloseKey(hkey) };
        }

        Ok(())
    }

    #[cfg(not(windows))]
    {
        Ok(())
    }
}
