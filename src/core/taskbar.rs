//! Windows Taskbar Progress indicator integration via Win32 COM / ITaskbarList3.

#[cfg(windows)]
use windows_sys::Win32::Foundation::HWND;

/// Status states for the Windows Taskbar icon progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarState {
    /// No progress displayed on the taskbar icon.
    NoProgress = 0,
    /// Indeterminate marquee animation.
    Indeterminate = 1,
    /// Normal green progress bar.
    Normal = 2,
    /// Error red progress bar.
    Error = 4,
    /// Paused yellow progress bar.
    Paused = 8,
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_snake_case)]
struct ITaskbarList3Vtbl {
    // IUnknown
    pub QueryInterface: unsafe extern "system" fn(
        *mut std::ffi::c_void,
        *const windows_sys::core::GUID,
        *mut *mut std::ffi::c_void,
    ) -> i32,
    pub AddRef: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
    pub Release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,

    // ITaskbarList
    pub HrInit: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
    pub AddTab: unsafe extern "system" fn(*mut std::ffi::c_void, HWND) -> i32,
    pub DeleteTab: unsafe extern "system" fn(*mut std::ffi::c_void, HWND) -> i32,
    pub ActivateTab: unsafe extern "system" fn(*mut std::ffi::c_void, HWND) -> i32,
    pub SetActiveAlt: unsafe extern "system" fn(*mut std::ffi::c_void, HWND) -> i32,

    // ITaskbarList2
    pub MarkFullscreenWindow: unsafe extern "system" fn(*mut std::ffi::c_void, HWND, i32) -> i32,

    // ITaskbarList3
    pub SetProgressValue: unsafe extern "system" fn(*mut std::ffi::c_void, HWND, u64, u64) -> i32,
    pub SetProgressState: unsafe extern "system" fn(*mut std::ffi::c_void, HWND, i32) -> i32,
}

/// Manages taskbar progress indicator on Windows.
pub struct TaskbarProgress {
    #[cfg(windows)]
    taskbar: Option<*mut std::ffi::c_void>,
}

unsafe impl Send for TaskbarProgress {}
unsafe impl Sync for TaskbarProgress {}

impl TaskbarProgress {
    /// Initializes the Taskbar progress helper.
    #[must_use]
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Com::{
                CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
            };

            // CLSID_TaskbarList: {56FDF344-FD6D-11D0-958A-006097C9A090}
            let clsid_taskbar_list =
                windows_sys::core::GUID::from_u128(0x56FDF344_FD6D_11D0_958A_006097C9A090);
            // IID_ITaskbarList3: {EA1AFB91-9E28-4B86-90E9-9E9F8A5EEFAF}
            let iid_itaskbar_list3 =
                windows_sys::core::GUID::from_u128(0xEA1AFB91_9E28_4B86_90E9_9E9F8A5EEFAF);

            unsafe {
                let _ = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
                let mut taskbar: *mut std::ffi::c_void = std::ptr::null_mut();
                let hr = CoCreateInstance(
                    &clsid_taskbar_list,
                    std::ptr::null_mut(),
                    CLSCTX_INPROC_SERVER,
                    &iid_itaskbar_list3,
                    &mut taskbar,
                );

                if hr == 0 && !taskbar.is_null() {
                    let vtbl = *(taskbar as *mut *mut ITaskbarList3Vtbl);
                    if let Some(vtbl_ref) = vtbl.as_ref() {
                        let hr_init = (vtbl_ref.HrInit)(taskbar);
                        if hr_init == 0 {
                            return Self {
                                taskbar: Some(taskbar),
                            };
                        }
                    }
                }
            }
            Self { taskbar: None }
        }

        #[cfg(not(windows))]
        {
            Self {}
        }
    }

    /// Updates taskbar progress percentage and state.
    pub fn update(&self, _hwnd: isize, _completed: u64, _total: u64, _state: TaskbarState) {
        #[cfg(windows)]
        {
            let hwnd = _hwnd as HWND;
            if hwnd.is_null() {
                return;
            }

            if let Some(tbl) = self.taskbar {
                unsafe {
                    let vtbl = *(tbl as *mut *mut ITaskbarList3Vtbl);
                    if let Some(vtbl_ref) = vtbl.as_ref() {
                        (vtbl_ref.SetProgressState)(tbl, hwnd, _state as i32);
                        if _total > 0 && _state != TaskbarState::NoProgress {
                            (vtbl_ref.SetProgressValue)(tbl, hwnd, _completed, _total);
                        }
                    }
                }
            }
        }
    }
}

impl Drop for TaskbarProgress {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            if let Some(tbl) = self.taskbar {
                unsafe {
                    let vtbl = *(tbl as *mut *mut ITaskbarList3Vtbl);
                    if let Some(vtbl_ref) = vtbl.as_ref() {
                        (vtbl_ref.Release)(tbl);
                    }
                }
            }
        }
    }
}

impl Default for TaskbarProgress {
    fn default() -> Self {
        Self::new()
    }
}
