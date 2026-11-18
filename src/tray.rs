//! System tray icon and configuration context menu.

use crate::blur::GlassTheme;
use crate::config::{Config, GroupingMode, SidebarEdge, TriggerMode};
use log::{debug, info, warn};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_TRAY_ICON: u32 = WM_USER + 100;

pub const MENU_TOGGLE_SIDEBAR: u32 = 1001;
pub const MENU_AUTO_HIDE: u32 = 1002;
pub const MENU_TRIGGER_TOP_CORNER: u32 = 1003;
pub const MENU_TRIGGER_BOTTOM_CORNER: u32 = 1004;
pub const MENU_TRIGGER_FULL_EDGE: u32 = 1005;
pub const MENU_BLUR_BACKDROP: u32 = 1006;
pub const MENU_THEME_DARK: u32 = 1007;
pub const MENU_THEME_LIGHT: u32 = 1008;
pub const MENU_EDGE_LEFT: u32 = 1009;
pub const MENU_EDGE_RIGHT: u32 = 1010;
pub const MENU_GROUP_BY_APP: u32 = 1011;
pub const MENU_GROUP_SINGLE: u32 = 1012;
pub const MENU_INCLUDE_ACTIVE: u32 = 1013;
pub const MENU_RESTORE_ALL: u32 = 1014;
pub const MENU_EXIT: u32 = 1015;
pub const MENU_RUN_AT_STARTUP: u32 = 1016;

pub struct TrayIcon {
    hwnd: HWND,
    icon_id: u32,
    owned_hicon: Option<HICON>,
}

impl TrayIcon {
    pub fn new(hwnd: HWND) -> Option<Self> {
        let icon_id = 1;
        let tip = wide_fixed::<128>("Stage Manager — Click to toggle, Right-click for options");
        let custom = create_stage_manager_hicon(32);
        let icon = match custom {
            Some(h) => h,
            None => unsafe { LoadIconW(None, IDI_APPLICATION).ok()? },
        };

        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: icon_id,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAY_ICON,
            hIcon: icon,
            szTip: tip,
            ..Default::default()
        };

        let ok = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() };
        if ok {
            info!("System tray icon registered");
            Some(Self {
                hwnd,
                icon_id,
                owned_hicon: custom,
            })
        } else {
            if let Some(h) = custom {
                unsafe {
                    let _ = DestroyIcon(h);
                }
            }
            warn!("Failed to register system tray icon");
            None
        }
    }

    pub fn show_context_menu(
        &self,
        config: &Config,
        sidebar_revealed: bool,
        startup_enabled: bool,
    ) -> Option<u32> {
        unsafe {
            let Ok(menu) = CreatePopupMenu() else {
                return None;
            };

            let toggle_label = if sidebar_revealed {
                "Hide Stage Manager (Win+` / Alt+`)"
            } else {
                "Show Stage Manager (Win+` / Alt+`)"
            };
            append_item(menu, MENU_TOGGLE_SIDEBAR, toggle_label, false);
            append_item(
                menu,
                MENU_AUTO_HIDE,
                "Auto-Hide (Side Edge Trigger)",
                config.auto_hide,
            );

            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

            append_item(
                menu,
                MENU_TRIGGER_FULL_EDGE,
                "Trigger: Any Point on Side",
                config.trigger_mode == TriggerMode::FullEdge,
            );
            append_item(
                menu,
                MENU_TRIGGER_TOP_CORNER,
                "Trigger: Top Corner Only",
                config.trigger_mode == TriggerMode::TopCorner,
            );
            append_item(
                menu,
                MENU_TRIGGER_BOTTOM_CORNER,
                "Trigger: Bottom Corner Only",
                config.trigger_mode == TriggerMode::BottomCorner,
            );

            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

            append_item(
                menu,
                MENU_BLUR_BACKDROP,
                "Show Frosted Backdrop Box",
                config.blur_backdrop,
            );
            append_item(
                menu,
                MENU_THEME_DARK,
                "Dark Frosted Blur",
                config.glass_theme == GlassTheme::DarkFrosted,
            );
            append_item(
                menu,
                MENU_THEME_LIGHT,
                "Light Frosted Blur",
                config.glass_theme == GlassTheme::LightFrosted,
            );

            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

            append_item(
                menu,
                MENU_EDGE_LEFT,
                "Left Side",
                config.sidebar_edge == SidebarEdge::Left,
            );
            append_item(
                menu,
                MENU_EDGE_RIGHT,
                "Right Side",
                config.sidebar_edge == SidebarEdge::Right,
            );

            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

            append_item(
                menu,
                MENU_GROUP_BY_APP,
                "Group Windows by App",
                config.grouping_mode == GroupingMode::ByApplication,
            );
            append_item(
                menu,
                MENU_GROUP_SINGLE,
                "One Window per Stage",
                config.grouping_mode == GroupingMode::SingleWindow,
            );
            append_item(
                menu,
                MENU_INCLUDE_ACTIVE,
                "Show Current App in Strip",
                config.include_active_app,
            );

            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            append_item(
                menu,
                MENU_RUN_AT_STARTUP,
                "Run at Windows Startup",
                startup_enabled,
            );
            append_item(menu, MENU_RESTORE_ALL, "Restore All Windows", false);
            append_item(menu, MENU_EXIT, "Exit Stage Manager", false);

            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            let _ = SetForegroundWindow(self.hwnd);
            let selected = TrackPopupMenu(
                menu,
                TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
                pt.x,
                pt.y,
                0,
                self.hwnd,
                None,
            );
            let _ = PostMessageW(self.hwnd, WM_NULL, WPARAM(0), LPARAM(0));
            let _ = DestroyMenu(menu);
            if selected.0 > 0 {
                Some(selected.0 as u32)
            } else {
                None
            }
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: self.icon_id,
            ..Default::default()
        };
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            if let Some(h) = self.owned_hicon.take() {
                let _ = DestroyIcon(h);
            }
        }
        debug!("System tray icon removed");
    }
}

/// Generate a crisp 32-bit ARGB Stage Manager tray icon (rounded dark-glass squircle + mini cards + active window).
fn create_stage_manager_hicon(size: i32) -> Option<HICON> {
    let mut pixels = vec![0u32; (size * size) as usize];
    let s = size as f32;
    let r = s * 0.24;

    let in_rounded_rect = |x: f32, y: f32, l: f32, t: f32, ri: f32, b: f32, rad: f32| -> f32 {
        let cx = ((l + rad) - x).max(0.0).max(x - (ri - rad));
        let cy = ((t + rad) - y).max(0.0).max(y - (b - rad));
        let dist = (cx * cx + cy * cy).sqrt() - rad;
        (0.5 - dist).clamp(0.0, 1.0)
    };

    let blend = |dst: u32, r_c: f32, g_c: f32, b_c: f32, a_c: f32| -> u32 {
        if a_c <= 0.001 {
            return dst;
        }
        let da = ((dst >> 24) & 0xFF) as f32 / 255.0;
        let dr = ((dst >> 16) & 0xFF) as f32 / 255.0;
        let dg = ((dst >> 8) & 0xFF) as f32 / 255.0;
        let db = (dst & 0xFF) as f32 / 255.0;

        let out_a = (a_c + da * (1.0 - a_c)).clamp(0.0, 1.0);
        if out_a <= 0.0001 {
            return 0;
        }
        let out_r = (r_c * a_c + dr * da * (1.0 - a_c)) / out_a;
        let out_g = (g_c * a_c + dg * da * (1.0 - a_c)) / out_a;
        let out_b = (b_c * a_c + db * da * (1.0 - a_c)) / out_a;

        (((out_a * 255.0).round() as u32) << 24)
            | (((out_r * 255.0).round() as u32) << 16)
            | (((out_g * 255.0).round() as u32) << 8)
            | ((out_b * 255.0).round() as u32)
    };

    for py in 0..size {
        for px in 0..size {
            let x = px as f32 + 0.5;
            let y = py as f32 + 0.5;
            let idx = (py * size + px) as usize;

            // Squircle dark indigo-blue gradient background
            let bg_cov = in_rounded_rect(x, y, 1.0, 1.0, s - 1.0, s - 1.0, r);
            let t = (y / s).clamp(0.0, 1.0);
            let mut c = blend(
                0,
                0.13 + 0.05 * (1.0 - t),
                0.26 + 0.16 * (1.0 - t),
                0.52 + 0.24 * (1.0 - t),
                bg_cov,
            );

            // Three stacked stage cards along left edge
            for (i, (top_f, bot_f)) in [(0.20, 0.36), (0.42, 0.58), (0.64, 0.80)]
                .iter()
                .enumerate()
            {
                let card_cov =
                    in_rounded_rect(x, y, s * 0.16, s * top_f, s * 0.36, s * bot_f, s * 0.04);
                let bright = if i == 1 { 0.96 } else { 0.78 };
                c = blend(c, bright, bright + 0.02, 1.0, card_cov * 0.95);
            }

            // Main active window card on the right
            let main_cov = in_rounded_rect(x, y, s * 0.43, s * 0.22, s * 0.84, s * 0.78, s * 0.09);
            c = blend(c, 0.96, 0.98, 1.0, main_cov * 0.98);
            pixels[idx] = c;
        }
    }

    unsafe {
        let hdc = CreateCompatibleDC(None);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let Ok(color_bmp) = CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
            let _ = DeleteDC(hdc);
            return None;
        };
        if !bits.is_null() {
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u32, pixels.len());
        }
        let mask_bmp: HBITMAP = CreateBitmap(size, size, 1, 1, None);
        let ii = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask_bmp,
            hbmColor: color_bmp,
        };
        let icon = CreateIconIndirect(&ii).ok();
        let _ = DeleteObject(color_bmp);
        let _ = DeleteObject(mask_bmp);
        let _ = DeleteDC(hdc);
        icon
    }
}

unsafe fn append_item(menu: HMENU, id: u32, text: &str, checked: bool) {
    let flags = if checked {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    let w = wide(text);
    let _ = AppendMenuW(menu, flags, id as usize, PCWSTR(w.as_ptr()));
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn wide_fixed<const N: usize>(s: &str) -> [u16; N] {
    let mut buf = [0u16; N];
    for (i, c) in OsStr::new(s).encode_wide().enumerate() {
        if i >= N - 1 {
            break;
        }
        buf[i] = c;
    }
    buf
}
