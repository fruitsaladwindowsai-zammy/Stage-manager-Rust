//! Window discovery, filtering, DWM bounds/cropping, icon lookup, and WinEvent hooks.

use crate::desktop::{monitor_for_window, DesktopId, VirtualDesktopTracker};
use log::{debug, warn};
use std::collections::HashMap;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, MAX_PATH, RECT, TRUE, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::*;

/// Custom Win32 message posted when a watched window event occurs.
pub const WM_STAGE_WIN_EVENT: u32 = WM_APP + 11;

static EVENT_TARGET_HWND: AtomicIsize = AtomicIsize::new(0);

/// Invisible DWM drop-shadow/resize-border insets around a window's visual frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameInsets {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Metadata for a single tracked application window.
#[derive(Debug, Clone)]
pub struct TrackedWindow {
    pub hwnd: HWND,
    pub hwnd_key: isize,
    pub title: String,
    pub exe_name: String,
    pub exe_path: PathBuf,
    /// Visual frame bounds (from `DWMWA_EXTENDED_FRAME_BOUNDS`) when not minimized.
    pub visual_rect: RECT,
    /// Raw `GetWindowRect` when not minimized.
    pub window_rect: RECT,
    /// Cached invisible resize border insets so DWM thumbnail cropping stays crisp even when minimized.
    pub frame_insets: FrameInsets,
    /// Cached normal/restored rect from `WINDOWPLACEMENT`.
    pub normal_rect: RECT,
    pub was_maximized: bool,
    pub is_minimized: bool,
    pub icon: Option<HICON>,
    /// True if Stage Manager owns/created icon via `SHGetFileInfoW` and should `DestroyIcon` when dropped?
    /// We keep raw HICON handles lightweight; icon textures are cached in the renderer.
    pub owns_icon: bool,
    /// HMONITOR handle key (`HMONITOR.0 as isize`) owning this window.
    pub monitor_key: isize,
    /// Windows Virtual Desktop GUID bytes (or `[0; 16]` if unavailable/pinned).
    pub desktop_id: DesktopId,
}

impl TrackedWindow {
    /// Best estimate of the window's on-screen visual rect when restored.
    pub fn desktop_target_rect(&self, work_area: &RECT) -> RECT {
        if self.was_maximized {
            *work_area
        } else if self.normal_rect.right > self.normal_rect.left
            && self.normal_rect.bottom > self.normal_rect.top
        {
            self.normal_rect
        } else if self.visual_rect.right > self.visual_rect.left
            && self.visual_rect.bottom > self.visual_rect.top
        {
            self.visual_rect
        } else {
            RECT {
                left: work_area.left + 160,
                top: work_area.top + 80,
                right: work_area.right - 160,
                bottom: work_area.bottom - 80,
            }
        }
    }
}

/// Enumerates and monitors top-level application windows.
pub struct WindowTracker {
    windows: HashMap<isize, TrackedWindow>,
    hooks: Vec<HWINEVENTHOOK>,
    desktop: VirtualDesktopTracker,
}

impl WindowTracker {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            hooks: Vec::new(),
            desktop: VirtualDesktopTracker::new(),
        }
    }

    /// Perform a full synchronization pass against `EnumWindows` for the current virtual desktop.
    /// Preserves cached visual bounds / frame insets for windows that are currently minimized.
    pub fn sync_all(&mut self) -> Vec<TrackedWindow> {
        let mut discovered: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(
                Some(enum_windows_cb),
                LPARAM(&mut discovered as *mut Vec<HWND> as isize),
            );
        }

        let mut next_map: HashMap<isize, TrackedWindow> = HashMap::with_capacity(discovered.len());
        let mut ordered: Vec<TrackedWindow> = Vec::with_capacity(discovered.len());

        for hwnd in discovered {
            if !self.desktop.is_on_current_desktop(hwnd) {
                continue;
            }
            let key = hwnd.0 as isize;
            let prev = self.windows.get(&key);
            if let Some(mut info) = query_window_info(hwnd, prev) {
                info.desktop_id = self.desktop.window_desktop_id(hwnd);
                ordered.push(info.clone());
                next_map.insert(key, info);
            }
        }

        self.windows = next_map;
        ordered
    }

    /// Install out-of-context WinEvent hooks delivering events to `notify_hwnd`.
    pub fn install_hooks(&mut self, notify_hwnd: HWND) {
        EVENT_TARGET_HWND.store(notify_hwnd.0 as isize, Ordering::Release);

        let ranges = [
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
            (EVENT_OBJECT_CREATE, EVENT_OBJECT_HIDE),
            (EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE),
        ];

        for (min, max) in ranges {
            let hook = unsafe {
                SetWinEventHook(
                    min,
                    max,
                    None,
                    Some(win_event_cb),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                )
            };
            if !hook.is_invalid() {
                self.hooks.push(hook);
            } else {
                warn!("Failed to install WinEventHook for {min:#x}..{max:#x}");
            }
        }
        debug!("Installed {} WinEvent hooks", self.hooks.len());
    }
}

impl Drop for WindowTracker {
    fn drop(&mut self) {
        EVENT_TARGET_HWND.store(0, Ordering::Release);
        for hook in &self.hooks {
            unsafe {
                let _ = UnhookWinEvent(*hook);
            }
        }
    }
}

/// Check if an `HWND` represents a real user-facing top-level application window.
pub fn should_track_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindow(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return false;
        }

        let style = WINDOW_STYLE(GetWindowLongW(hwnd, GWL_STYLE) as u32);
        let ex_style = WINDOW_EX_STYLE(GetWindowLongW(hwnd, GWL_EXSTYLE) as u32);

        if ex_style.contains(WS_EX_TOOLWINDOW) {
            return false;
        }

        if ex_style.contains(WS_EX_NOACTIVATE) && !ex_style.contains(WS_EX_APPWINDOW) {
            return false;
        }

        if !style.contains(WS_VISIBLE) {
            return false;
        }

        let title_len = GetWindowTextLengthW(hwnd);
        if title_len <= 0 {
            return false;
        }

        // Skip cloaked windows (other virtual desktops, suspended UWP, Start/Search hosts)
        let mut cloaked: u32 = 0;
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut _,
            std::mem::size_of::<u32>() as u32,
        );
        if hr.is_ok() && cloaked != 0 {
            return false;
        }

        // Skip child/owned popup dialogs unless explicitly marked WS_EX_APPWINDOW
        if let Ok(owner) = GetWindow(hwnd, GW_OWNER) {
            if !owner.0.is_null() && !ex_style.contains(WS_EX_APPWINDOW) {
                return false;
            }
        }

        // Skip Windows shell classes
        let class_name = get_class_name(hwnd);
        if is_ignored_shell_class(&class_name) {
            return false;
        }

        // If not minimized, require reasonable minimum dimensions (skip hidden 0x0 or 1x1 helpers)
        if !IsIconic(hwnd).as_bool() {
            let mut wr = RECT::default();
            if GetWindowRect(hwnd, &mut wr).is_ok() {
                let w = wr.right - wr.left;
                let h = wr.bottom - wr.top;
                if w < 80 || h < 50 {
                    return false;
                }
            }
        }

        true
    }
}

fn is_ignored_shell_class(class_name: &str) -> bool {
    matches!(
        class_name,
        "Progman"
            | "WorkerW"
            | "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "Windows.UI.Core.CoreWindow"
            | "TopLevelWindowForOverflowXamlIsland"
            | "Xaml_WindowedPopupClass"
            | "MultitaskingViewFrame"
            | "ForegroundStaging"
            | "TaskListThumbnailWnd"
    )
}

/// Query full details for a tracked window, reusing cached visual bounds when minimized.
pub fn query_window_info(hwnd: HWND, prev: Option<&TrackedWindow>) -> Option<TrackedWindow> {
    if !should_track_window(hwnd) {
        return None;
    }

    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buf);
        let title = OsString::from_wide(&buf[..copied as usize])
            .to_string_lossy()
            .to_string();

        if title.is_empty() {
            return None;
        }

        let is_minimized = IsIconic(hwnd).as_bool();

        let mut wp = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        let _ = GetWindowPlacement(hwnd, &mut wp);
        let was_maximized = if is_minimized {
            (wp.flags & WPF_RESTORETOMAXIMIZED) != WINDOWPLACEMENT_FLAGS(0)
                || prev.map(|p| p.was_maximized).unwrap_or(false)
        } else {
            wp.showCmd == SW_SHOWMAXIMIZED.0 as u32
        };

        let mut window_rect = prev.map(|p| p.window_rect).unwrap_or_default();
        let mut visual_rect = prev.map(|p| p.visual_rect).unwrap_or_default();
        let mut frame_insets = prev.map(|p| p.frame_insets).unwrap_or_default();

        if !is_minimized {
            let mut wr = RECT::default();
            if GetWindowRect(hwnd, &mut wr).is_ok() {
                window_rect = wr;
                let mut er = wr;
                let hr = DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_EXTENDED_FRAME_BOUNDS,
                    &mut er as *mut RECT as *mut _,
                    std::mem::size_of::<RECT>() as u32,
                );
                if hr.is_ok() {
                    visual_rect = er;
                    frame_insets = FrameInsets {
                        left: (er.left - wr.left).max(0),
                        top: (er.top - wr.top).max(0),
                        right: (wr.right - er.right).max(0),
                        bottom: (wr.bottom - er.bottom).max(0),
                    };
                } else {
                    visual_rect = wr;
                }
            }
        }

        // Reuse process exe info from previous entry if available to avoid extra OpenProcess calls
        let (exe_name, exe_path) = if let Some(p) = prev {
            if !p.exe_name.is_empty() {
                (p.exe_name.clone(), p.exe_path.clone())
            } else {
                query_exe_for_hwnd(hwnd)
            }
        } else {
            query_exe_for_hwnd(hwnd)
        };

        let (icon, owns_icon) = if let Some(p) = prev {
            if p.icon.is_some() {
                (p.icon, p.owns_icon)
            } else {
                query_window_icon(hwnd, &exe_path)
            }
        } else {
            query_window_icon(hwnd, &exe_path)
        };

        Some(TrackedWindow {
            hwnd,
            hwnd_key: hwnd.0 as isize,
            title,
            exe_name,
            exe_path,
            visual_rect,
            window_rect,
            frame_insets,
            normal_rect: wp.rcNormalPosition,
            was_maximized,
            is_minimized,
            icon,
            owns_icon,
            monitor_key: monitor_for_window(hwnd),
            desktop_id: [0u8; 16],
        })
    }
}

fn get_class_name(hwnd: HWND) -> String {
    unsafe {
        let mut buf = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut buf);
        if len > 0 {
            OsString::from_wide(&buf[..len as usize])
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        }
    }
}

fn query_exe_for_hwnd(hwnd: HWND) -> (String, PathBuf) {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return (String::new(), PathBuf::new());
        }

        if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; MAX_PATH as usize];
            let mut size = buf.len() as u32;
            let res = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut size,
            );
            let _ = CloseHandle(handle);

            if res.is_ok() && size > 0 {
                let path = PathBuf::from(OsString::from_wide(&buf[..size as usize]));
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                return (name, path);
            }
        }
    }
    (String::new(), PathBuf::new())
}

fn query_window_icon(hwnd: HWND, exe_path: &PathBuf) -> (Option<HICON>, bool) {
    unsafe {
        // 1. Ask the window directly (with short timeout so hung apps don't block us)
        for icon_type in [ICON_BIG as usize, ICON_SMALL2 as usize, ICON_SMALL as usize] {
            let mut result: usize = 0;
            let ok = SendMessageTimeoutW(
                hwnd,
                WM_GETICON,
                WPARAM(icon_type),
                LPARAM(0),
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                40,
                Some(&mut result),
            );
            if ok.0 != 0 && result != 0 {
                return (Some(HICON(result as *mut _)), false);
            }
        }

        // 2. Class icon
        let class_icon = GetClassLongPtrW(hwnd, GCLP_HICON);
        if class_icon != 0 {
            return (Some(HICON(class_icon as *mut _)), false);
        }
        let class_icon_sm = GetClassLongPtrW(hwnd, GCLP_HICONSM);
        if class_icon_sm != 0 {
            return (Some(HICON(class_icon_sm as *mut _)), false);
        }

        // 3. Executable shell icon fallback
        if !exe_path.as_os_str().is_empty() {
            let wide: Vec<u16> = exe_path
                .as_os_str()
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mut sfi = SHFILEINFOW::default();
            let res = SHGetFileInfoW(
                PCWSTR(wide.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut sfi),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            if res != 0 && !sfi.hIcon.is_invalid() {
                return (Some(sfi.hIcon), true);
            }
        }
    }
    (None, false)
}

unsafe extern "system" fn enum_windows_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<HWND>);
    if should_track_window(hwnd) {
        list.push(hwnd);
    }
    TRUE
}

unsafe extern "system" fn win_event_cb(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _event_time: u32,
) {
    if id_object != 0 || hwnd.0.is_null() {
        return;
    }
    let target_raw = EVENT_TARGET_HWND.load(Ordering::Acquire);
    if target_raw != 0 {
        let target = HWND(target_raw as *mut _);
        let _ = PostMessageW(
            target,
            WM_STAGE_WIN_EVENT,
            WPARAM(event as usize),
            LPARAM(hwnd.0 as isize),
        );
    }
}
