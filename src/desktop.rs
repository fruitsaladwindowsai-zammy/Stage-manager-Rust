//! Multi-monitor enumeration, Windows Virtual Desktop isolation, and Boot Startup registry helpers.

use log::warn;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, HDC, HMONITOR,
    MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Shell::{IVirtualDesktopManager, VirtualDesktopManager};
use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const STARTUP_VALUE_NAME: &str = "StageManager";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorInfo {
    pub monitor_key: isize,
    pub monitor_rect: RECT,
    pub work_area: RECT,
    pub dpi_scale: f64,
    pub is_primary: bool,
}

pub type DesktopId = [u8; 16];

/// Enumerate all connected display monitors with per-monitor work area and DPI scale.
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    let mut list: Vec<MonitorInfo> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_monitors_cb),
            LPARAM(&mut list as *mut Vec<MonitorInfo> as isize),
        );
    }
    if list.is_empty() {
        // Fallback single-monitor entry if EnumDisplayMonitors yields nothing
        list.push(MonitorInfo {
            monitor_key: 0,
            monitor_rect: RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            },
            work_area: RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1040,
            },
            dpi_scale: 1.0,
            is_primary: true,
        });
    }
    list.sort_by_key(|m| (!m.is_primary, m.monitor_rect.left, m.monitor_rect.top));
    list
}

pub fn monitor_for_window(hwnd: HWND) -> isize {
    unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST).0 as isize }
}

pub fn monitor_for_point(pt: POINT) -> isize {
    unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST).0 as isize }
}

unsafe extern "system" fn enum_monitors_cb(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rc: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let list = &mut *(data.0 as *mut Vec<MonitorInfo>);
    let mut mi = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };

    if GetMonitorInfoW(hmonitor, &mut mi.monitorInfo).as_bool() {
        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        let _ = GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let dpi_scale = (dpi_x.max(96) as f64) / 96.0;
        let is_primary = (mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0;

        list.push(MonitorInfo {
            monitor_key: hmonitor.0 as isize,
            monitor_rect: mi.monitorInfo.rcMonitor,
            work_area: mi.monitorInfo.rcWork,
            dpi_scale,
            is_primary,
        });
    }
    BOOL(1)
}

/// Thin wrapper over `IVirtualDesktopManager` for per-Virtual-Desktop isolation.
pub struct VirtualDesktopTracker {
    mgr: Option<IVirtualDesktopManager>,
}

impl VirtualDesktopTracker {
    pub fn new() -> Self {
        let mgr = unsafe {
            CoCreateInstance::<_, IVirtualDesktopManager>(&VirtualDesktopManager, None, CLSCTX_ALL)
                .ok()
        };
        if mgr.is_none() {
            warn!("IVirtualDesktopManager unavailable; falling back to DWM cloak filtering");
        }
        Self { mgr }
    }

    pub fn is_on_current_desktop(&self, hwnd: HWND) -> bool {
        let Some(ref mgr) = self.mgr else {
            return true;
        };
        unsafe {
            match mgr.IsWindowOnCurrentVirtualDesktop(hwnd) {
                Ok(b) => b.as_bool(),
                Err(_) => true,
            }
        }
    }

    pub fn window_desktop_id(&self, hwnd: HWND) -> DesktopId {
        let Some(ref mgr) = self.mgr else {
            return [0u8; 16];
        };
        unsafe {
            match mgr.GetWindowDesktopId(hwnd) {
                Ok(guid) => guid_to_bytes(&guid),
                Err(_) => [0u8; 16],
            }
        }
    }
}

fn guid_to_bytes(g: &GUID) -> DesktopId {
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&g.data1.to_le_bytes());
    out[4..6].copy_from_slice(&g.data2.to_le_bytes());
    out[6..8].copy_from_slice(&g.data3.to_le_bytes());
    out[8..16].copy_from_slice(&g.data4);
    out
}

/// Check if Stage Manager is configured to start automatically at Windows login.
pub fn is_run_at_startup_enabled() -> bool {
    let subkey = wide(RUN_SUBKEY);
    let val_name = wide(STARTUP_VALUE_NAME);
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_QUERY_VALUE,
            &mut hkey,
        )
        .is_err()
        {
            return false;
        }
        let exists =
            RegQueryValueExW(hkey, PCWSTR(val_name.as_ptr()), None, None, None, None).is_ok();
        let _ = RegCloseKey(hkey);
        exists
    }
}

/// Enable or disable starting the current executable at Windows login (`HKCU\...\Run`).
pub fn set_run_at_startup(enabled: bool) -> bool {
    let subkey = wide(RUN_SUBKEY);
    let val_name = wide(STARTUP_VALUE_NAME);
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        )
        .is_err()
        {
            return false;
        }

        let ok = if enabled {
            if let Ok(exe) = std::env::current_exe() {
                let quoted = format!("\"{}\"", exe.display());
                let w = wide(&quoted);
                let bytes = std::slice::from_raw_parts(w.as_ptr() as *const u8, w.len() * 2);
                RegSetValueExW(hkey, PCWSTR(val_name.as_ptr()), 0, REG_SZ, Some(bytes)).is_ok()
            } else {
                false
            }
        } else {
            let err = RegDeleteValueW(hkey, PCWSTR(val_name.as_ptr()));
            err.is_ok() || !is_run_at_startup_enabled()
        };

        let _ = RegCloseKey(hkey);
        ok
    }
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
