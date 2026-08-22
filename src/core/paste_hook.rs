//! Windows Explorer Ctrl+V low-level keyboard interceptor hook (TeraCopy-style).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

static IS_INTERCEPTOR_ENABLED: AtomicBool = AtomicBool::new(true);
static HOOK_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
#[derive(Clone, Copy)]
#[repr(C)]
struct KbdLlHookStruct {
    vk_code: u32,
    scan_code: u32,
    flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[cfg(windows)]
unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetClassNameW, GetForegroundWindow, WM_KEYDOWN, WM_SYSKEYDOWN,
    };

    if n_code >= 0
        && IS_INTERCEPTOR_ENABLED.load(Ordering::Relaxed)
        && (w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize)
    {
        let kbd = *(l_param as *const KbdLlHookStruct);
        const VK_V: u32 = 0x56;

        if kbd.vk_code == VK_V {
            let ctrl_down = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;

            if ctrl_down {
                let hwnd = GetForegroundWindow();
                if !hwnd.is_null() {
                    let mut class_buf = [0u16; 256];
                    let len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32);

                    if len > 0 {
                        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
                        let is_explorer = class_name == "CabinetWClass"
                            || class_name == "ExploreWClass"
                            || class_name == "Progman"
                            || class_name == "WorkerW";

                        if is_explorer && crate::core::clipboard::has_clipboard_files() {
                            // Spawn CopyFile prompt in a separate background thread
                            thread::spawn(|| {
                                let dest_folder =
                                    crate::core::clipboard::get_active_explorer_path()
                                        .unwrap_or_else(|| PathBuf::from("."));

                                let exe_path = if crate::core::shell::is_installed_in_windows() {
                                    PathBuf::from(r"C:\Windows\copyfile.exe")
                                } else {
                                    std::env::current_exe()
                                        .unwrap_or_else(|_| PathBuf::from("copyfile.exe"))
                                };

                                let _ = std::process::Command::new(exe_path)
                                    .arg("--prompt-paste")
                                    .arg(dest_folder)
                                    .spawn();
                            });

                            // RETURN 1 to EAT / SUPPRESS the keystroke:
                            // This prevents Windows Explorer from receiving Ctrl+V and prevents it from copying!
                            return 1;
                        }
                    }
                }
            }
        }
    }

    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}

/// Starts the background thread that intercepts `Ctrl+V` within Windows Explorer and Desktop.
pub fn start_paste_interceptor(is_enabled: Arc<AtomicBool>) {
    if HOOK_INITIALIZED.swap(true, Ordering::SeqCst) {
        return; // Already initialized
    }

    IS_INTERCEPTOR_ENABLED.store(is_enabled.load(Ordering::Relaxed), Ordering::Relaxed);

    thread::spawn(move || {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
                UnhookWindowsHookEx, MSG, WH_KEYBOARD_LL,
            };

            let hook = unsafe {
                SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(low_level_keyboard_proc),
                    std::ptr::null_mut(),
                    0,
                )
            };

            if !hook.is_null() {
                let mut msg: MSG = unsafe { std::mem::zeroed() };
                while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {
                    unsafe {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
                unsafe {
                    UnhookWindowsHookEx(hook);
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
