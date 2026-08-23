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
fn is_explorer_active_window(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::ProcessStatus::GetProcessImageFileNameW;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetWindowThreadProcessId};

    if hwnd.is_null() {
        return false;
    }

    // 1. Fast check by window class name
    let mut class_buf = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32) };
    if len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
        if class_name == "CabinetWClass"
            || class_name == "ExploreWClass"
            || class_name == "Progman"
            || class_name == "WorkerW"
            || class_name == "ShellTabWindowClass"
            || class_name == "DirectUIHWND"
            || class_name == "SHELLDLL_DefView"
            || class_name == "SysListView32"
        {
            return true;
        }
    }

    // 2. Comprehensive check by process executable name
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid == 0 {
        return false;
    }

    let h_proc = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if h_proc.is_null() {
        return false;
    }

    let mut buf = [0u16; 512];
    let len = unsafe { GetProcessImageFileNameW(h_proc, buf.as_mut_ptr(), buf.len() as u32) };
    unsafe { CloseHandle(h_proc) };

    if len > 0 {
        let path_str = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
        path_str.ends_with("explorer.exe")
    } else {
        false
    }
}

#[cfg(windows)]
unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetForegroundWindow, WM_KEYDOWN, WM_SYSKEYDOWN,
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
                let shift_down = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;
                let alt_down = (GetAsyncKeyState(VK_MENU as i32) as u16 & 0x8000) != 0;
                let win_down = (GetAsyncKeyState(VK_LWIN as i32) as u16 & 0x8000) != 0
                    || (GetAsyncKeyState(VK_RWIN as i32) as u16 & 0x8000) != 0;

                if shift_down || alt_down || win_down {
                    // Don't intercept modified shortcuts like Ctrl+Shift+V
                    // Fall through to CallNextHookEx
                } else {
                    let hwnd = GetForegroundWindow();
                    if is_explorer_active_window(hwnd)
                        && crate::core::clipboard::has_clipboard_files()
                    {
                        // Spawn CopyFile prompt in a separate background thread
                        thread::spawn(|| {
                            let dest_folder = crate::core::clipboard::get_active_explorer_path()
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
