//! Windows Desktop Window Manager (DWM) Acrylic & Mica blur integration.
//!
//! Provides three backdrop effects for the borderless window:
//! - `None` — Solid background, no transparency
//! - `Mica` — Windows 11 desktop wallpaper blur (opaque, sample of wallpaper)
//! - `Acrylic` — Windows 10/11 frosted glass effect (translucent, blurred)
//!
//! Uses `dwmapi.dll` for modern DWM attributes with a `user32.dll` fallback
//! for Windows 10 builds that don't support `DWMWA_SYSTEMBACKDROP_TYPE`.
//!
//! All FFI declarations are contained in the `win32` submodule and are
//! only compiled on Windows targets.

#[cfg(target_os = "windows")]
#[allow(dead_code, non_snake_case)]
mod win32 {
    pub use std::ffi::c_void;

    pub type HWND = *mut c_void;
    pub type HMODULE = *mut c_void;
    pub type FARPROC = *mut c_void;
    pub type DWORD = u32;
    pub type BOOL = i32;
    pub type HRESULT = i32;
    pub type LPCSTR = *const i8;
    pub type LPCVOID = *const c_void;

    #[repr(C)]
    pub struct MARGINS {
        pub cxLeftWidth: i32,
        pub cxRightWidth: i32,
        pub cyTopHeight: i32,
        pub cyBottomHeight: i32,
    }

    #[repr(C)]
    pub struct AccentPolicy {
        pub AccentState: u32,
        pub AccentFlags: u32,
        pub GradientColor: u32,
        pub AnimationId: u32,
    }

    #[repr(C)]
    pub struct WindowCompositionAttributeData {
        pub Attribute: u32,
        pub Data: *mut c_void,
        pub SizeOfData: usize,
    }

    extern "system" {
        pub fn LoadLibraryA(lpLibFileName: LPCSTR) -> HMODULE;
        pub fn GetProcAddress(hModule: HMODULE, lpProcName: LPCSTR) -> FARPROC;
        pub fn FindWindowA(lpClassName: LPCSTR, lpWindowName: LPCSTR) -> HWND;
        pub fn GetActiveWindow() -> HWND;
        pub fn GetForegroundWindow() -> HWND;
        pub fn GetCurrentProcessId() -> DWORD;
        pub fn GetWindowThreadProcessId(hwnd: HWND, lpdwProcessId: *mut DWORD) -> DWORD;
        pub fn EnumWindows(lpEnumFunc: Option<unsafe extern "system" fn(HWND, isize) -> BOOL>, lParam: isize) -> BOOL;
        pub fn IsWindowVisible(hwnd: HWND) -> BOOL;
    }

    pub static mut FOUND_HWND: HWND = std::ptr::null_mut();

    pub unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: isize) -> BOOL {
        let target_pid = lparam as DWORD;
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == target_pid && IsWindowVisible(hwnd) != 0 {
            FOUND_HWND = hwnd;
            return 0;
        }
        1
    }

    pub type FnDwmSetWindowAttribute = unsafe extern "system" fn(
        hwnd: HWND,
        dwAttribute: DWORD,
        pvAttribute: LPCVOID,
        cbAttribute: DWORD,
    ) -> HRESULT;

    pub type FnDwmExtendFrameIntoClientArea = unsafe extern "system" fn(
        hwnd: HWND,
        pMarInset: *const MARGINS,
    ) -> HRESULT;

    pub type FnSetWindowCompositionAttribute = unsafe extern "system" fn(
        hwnd: HWND,
        data: *mut WindowCompositionAttributeData,
    ) -> BOOL;

    pub const DWMWA_USE_IMMERSIVE_DARK_MODE: DWORD = 20;
    pub const DWMWA_SYSTEMBACKDROP_TYPE: DWORD = 38;
    pub const DWMWA_MICA_EFFECT: DWORD = 1029;

    pub const DWMSBT_AUTO: u32 = 0;
    pub const DWMSBT_NONE: u32 = 1;
    pub const DWMSBT_MAINWINDOW: u32 = 2; // Mica
    pub const DWMSBT_TRANSIENTWINDOW: u32 = 3; // Acrylic
    pub const DWMSBT_TABBEDWINDOW: u32 = 4; // Mica Alt

    pub const ACCENT_DISABLED: u32 = 0;
    pub const ACCENT_ENABLE_GRADIENT: u32 = 1;
    pub const ACCENT_ENABLE_TRANSPARENTGRADIENT: u32 = 2;
    pub const ACCENT_ENABLE_BLURBEHIND: u32 = 3;
    pub const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;
}

/// Available window backdrop blur effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurEffect {
    /// No blur — solid window background
    None,
    /// Mica effect — samples desktop wallpaper (Windows 11+)
    Mica,
    /// Acrylic effect — frosted glass blur (Windows 10/11)
    Acrylic,
}

impl BlurEffect {
    pub fn name(&self) -> &'static str {
        match self {
            BlurEffect::None => "None",
            BlurEffect::Mica => "Mica",
            BlurEffect::Acrylic => "Acrylic",
        }
    }
}

/// Applies DWM blur / acrylic backdrop to the application window on Windows 11/10.
///
/// Process:
/// 1. Finds the window handle via PID enumeration, falling back to
///    `GetActiveWindow` → `GetForegroundWindow` → `FindWindowA`
/// 2. Loads `dwmapi.dll` and calls `DwmExtendFrameIntoClientArea`
/// 3. Sets `DWMWA_SYSTEMBACKDROP_TYPE` for the requested effect
/// 4. Falls back to `SetWindowCompositionAttribute` on older Windows 10
///
/// All operations are best-effort — failures are silently ignored.
pub fn apply_window_blur(_effect: BlurEffect) {
    #[cfg(target_os = "windows")]
    unsafe {
        let effect = _effect;
        use win32::*;

        // Find window handle for our process first
        let current_pid = GetCurrentProcessId();
        FOUND_HWND = std::ptr::null_mut();
        EnumWindows(Some(enum_window_callback), current_pid as isize);
        let mut hwnd = FOUND_HWND;
        if hwnd.is_null() {
            hwnd = GetActiveWindow();
        }
        if hwnd.is_null() {
            hwnd = GetForegroundWindow();
        }
        if hwnd.is_null() {
            let window_title = b"mindforge\0";
            hwnd = FindWindowA(std::ptr::null(), window_title.as_ptr() as LPCSTR);
        }
        if hwnd.is_null() {
            return;
        }

        let mut dwm_success = false;
        let dwmapi = LoadLibraryA(b"dwmapi.dll\0".as_ptr() as LPCSTR);
        if !dwmapi.is_null() {
            // 1. Extend frame into client area so DWM backdrop renders across client rect
            let extend_proc = GetProcAddress(dwmapi, b"DwmExtendFrameIntoClientArea\0".as_ptr() as LPCSTR);
            if !extend_proc.is_null() {
                let extend_frame: FnDwmExtendFrameIntoClientArea = std::mem::transmute(extend_proc);
                // Use zero margins on a decorations=false window.
                // Margins of -1 (sheet-of-glass) cause DWM to render its own
                // native NC caption buttons through any translucent titlebar,
                // creating ghost duplicates behind our custom buttons.
                let margins = MARGINS {
                    cxLeftWidth: 0,
                    cxRightWidth: 0,
                    cyTopHeight: 0,
                    cyBottomHeight: 0,
                };
                let _ = extend_frame(hwnd, &margins);
            }

            // 2. Set DWM window attributes
            let set_attr_proc = GetProcAddress(dwmapi, b"DwmSetWindowAttribute\0".as_ptr() as LPCSTR);
            if !set_attr_proc.is_null() {
                let set_attr: FnDwmSetWindowAttribute = std::mem::transmute(set_attr_proc);

                // Enable dark mode window frame
                let dark_mode: BOOL = 1;
                let _ = set_attr(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &dark_mode as *const BOOL as LPCVOID,
                    std::mem::size_of::<BOOL>() as DWORD,
                );

                let hr = match effect {
                    BlurEffect::Acrylic => {
                        let backdrop_type: u32 = DWMSBT_TRANSIENTWINDOW;
                        let r = set_attr(
                            hwnd,
                            DWMWA_SYSTEMBACKDROP_TYPE,
                            &backdrop_type as *const u32 as LPCVOID,
                            std::mem::size_of::<u32>() as DWORD,
                        );
                        if r != 0 {
                            let mica: BOOL = 1;
                            set_attr(
                                hwnd,
                                DWMWA_MICA_EFFECT,
                                &mica as *const BOOL as LPCVOID,
                                std::mem::size_of::<BOOL>() as DWORD,
                            )
                        } else {
                            0
                        }
                    }
                    BlurEffect::Mica => {
                        let backdrop_type: u32 = DWMSBT_MAINWINDOW;
                        let r = set_attr(
                            hwnd,
                            DWMWA_SYSTEMBACKDROP_TYPE,
                            &backdrop_type as *const u32 as LPCVOID,
                            std::mem::size_of::<u32>() as DWORD,
                        );
                        if r != 0 {
                            let mica: BOOL = 1;
                            set_attr(
                                hwnd,
                                DWMWA_MICA_EFFECT,
                                &mica as *const BOOL as LPCVOID,
                                std::mem::size_of::<BOOL>() as DWORD,
                            )
                        } else {
                            0
                        }
                    }
                    BlurEffect::None => {
                        let backdrop_type: u32 = DWMSBT_NONE;
                        set_attr(
                            hwnd,
                            DWMWA_SYSTEMBACKDROP_TYPE,
                            &backdrop_type as *const u32 as LPCVOID,
                            std::mem::size_of::<u32>() as DWORD,
                        )
                    }
                };
                if hr == 0 {
                    dwm_success = true;
                }
            }
        }

        // 3. Fallback for Windows 10 only if DWM modern system backdrop is unsupported
        if !dwm_success {
            let user32 = LoadLibraryA(b"user32.dll\0".as_ptr() as LPCSTR);
            if !user32.is_null() {
                let set_comp_proc = GetProcAddress(user32, b"SetWindowCompositionAttribute\0".as_ptr() as LPCSTR);
                if !set_comp_proc.is_null() {
                    let set_comp: FnSetWindowCompositionAttribute = std::mem::transmute(set_comp_proc);
                    let (accent_state, gradient_color) = match effect {
                        BlurEffect::Acrylic => (ACCENT_ENABLE_ACRYLICBLURBEHIND, 0x01181818),
                        BlurEffect::Mica => (ACCENT_ENABLE_BLURBEHIND, 0),
                        BlurEffect::None => (ACCENT_DISABLED, 0),
                    };
                    let mut policy = AccentPolicy {
                        AccentState: accent_state,
                        AccentFlags: 2,
                        GradientColor: gradient_color,
                        AnimationId: 0,
                    };
                    let mut data = WindowCompositionAttributeData {
                        Attribute: 19, // WCA_ACCENT_POLICY
                        Data: &mut policy as *mut _ as *mut c_void,
                        SizeOfData: std::mem::size_of::<AccentPolicy>(),
                    };
                    let _ = set_comp(hwnd, &mut data);
                }
            }
        }
    }
}
