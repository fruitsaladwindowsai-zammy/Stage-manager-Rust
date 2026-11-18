//! Windows 10/11 always-on Gaussian Acrylic blur and rounded DWM preview helpers.

use std::ffi::c_void;
use windows::core::{s, w};
use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMSBT_TRANSIENTWINDOW,
    DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::Controls::MARGINS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlassTheme {
    DarkFrosted,
    LightFrosted,
}

const WCA_ACCENT_POLICY: u32 = 19;
const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;
const ACCENT_FLAG_ENABLE_GRADIENT: u32 = 2;

#[repr(C)]
struct AccentPolicy {
    accent_state: u32,
    accent_flags: u32,
    gradient_color: u32, // 0xAABBGGRR
    animation_id: u32,
}

#[repr(C)]
struct WindowCompositionAttribData {
    attrib: u32,
    pv_data: *mut c_void,
    cb_data: usize,
}

type SetWindowCompositionAttributeFn =
    unsafe extern "system" fn(HWND, *mut WindowCompositionAttribData) -> BOOL;

/// Apply always-on macOS frosted Gaussian background blur + Win11 rounded corners.
pub fn apply_macos_glass_blur(hwnd: HWND, theme: GlassTheme) {
    unsafe {
        let dark: BOOL = match theme {
            GlassTheme::DarkFrosted => true.into(),
            GlassTheme::LightFrosted => false.into(),
        };
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark as *const BOOL as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );

        let corner = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner as *const _ as *const _,
            std::mem::size_of_val(&corner) as u32,
        );

        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

        // Light tint alpha (~22%) so deep desktop/window blur dominates like macOS vibrancy
        let gradient_color: u32 = match theme {
            GlassTheme::DarkFrosted => 0x38_18_16_14,
            GlassTheme::LightFrosted => 0x3C_F2_F5_F8,
        };

        if let Some(set_wca) = load_set_window_composition_attribute() {
            let mut policy = AccentPolicy {
                accent_state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
                accent_flags: ACCENT_FLAG_ENABLE_GRADIENT,
                gradient_color,
                animation_id: 0,
            };
            let mut data = WindowCompositionAttribData {
                attrib: WCA_ACCENT_POLICY,
                pv_data: &mut policy as *mut AccentPolicy as *mut c_void,
                cb_data: std::mem::size_of::<AccentPolicy>(),
            };
            let _ = set_wca(hwnd, &mut data);
        } else {
            let backdrop = DWMSBT_TRANSIENTWINDOW;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &backdrop as *const _ as *const _,
                std::mem::size_of_val(&backdrop) as u32,
            );
        }
    }
}

/// Give a floating live preview HWND native Windows 11 rounded corners + drop shadow.
pub fn apply_rounded_preview_style(hwnd: HWND) {
    unsafe {
        let corner = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner as *const _ as *const _,
            std::mem::size_of_val(&corner) as u32,
        );
    }
}

unsafe fn load_set_window_composition_attribute() -> Option<SetWindowCompositionAttributeFn> {
    let user32 = GetModuleHandleW(w!("user32.dll")).ok()?;
    let proc = GetProcAddress(user32, s!("SetWindowCompositionAttribute"))?;
    Some(std::mem::transmute::<_, SetWindowCompositionAttributeFn>(
        proc,
    ))
}
