//! Click-through DWM Thumbnail Transition Overlay for smooth macOS-style
//! window swoop animations when switching between stages.

use crate::animation::{Spring, SpringRect};
use crate::thumbnail::ThumbnailManager;
use crate::window_tracker::{FrameInsets, TrackedWindow};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, FillRect, GetStockObject, InvalidateRect, UpdateWindow, BLACK_BRUSH,
    HBRUSH, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::*;

struct AnimatingWindow {
    hwnd_key: isize,
    insets: Option<FrameInsets>,
    rect_spring: SpringRect,
    opacity_spring: Spring,
    is_incoming: bool,
}

pub struct TransitionOverlay {
    pub hwnd: HWND,
    screen_origin_x: i32,
    screen_origin_y: i32,
    thumbnails: ThumbnailManager,
    items: Vec<AnimatingWindow>,
    pending_restore: Vec<(isize, bool)>, // (hwnd_key, was_maximized)
    restored_real_windows: bool,
    linger_frames: u8,
    active: bool,
}

impl TransitionOverlay {
    pub fn new() -> windows::core::Result<Self> {
        let hinstance = unsafe { GetModuleHandleW(None)? };
        let class_name = wide("StageManagerTransitionOverlay");

        unsafe {
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(overlay_wnd_proc),
                hInstance: hinstance.into(),
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };
            let _ = RegisterClassExW(&wc);
        }

        let (vx, vy, vw, vh) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN).max(GetSystemMetrics(SM_CXSCREEN)),
                GetSystemMetrics(SM_CYSCREEN).max(GetSystemMetrics(SM_CYVIRTUALSCREEN)),
            )
        };

        let hwnd = unsafe {
            let h = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(wide("Stage Manager Transition").as_ptr()),
                WS_POPUP,
                vx,
                vy,
                vw,
                vh,
                None,
                None,
                HINSTANCE::from(hinstance),
                None,
            )?;

            let _ = SetLayeredWindowAttributes(h, COLORREF(0), 255, LWA_ALPHA);
            let margins = MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            };
            let _ = DwmExtendFrameIntoClientArea(h, &margins);
            h
        };

        Ok(Self {
            hwnd,
            screen_origin_x: vx,
            screen_origin_y: vy,
            thumbnails: ThumbnailManager::new(hwnd),
            items: Vec::new(),
            pending_restore: Vec::new(),
            restored_real_windows: false,
            linger_frames: 0,
            active: false,
        })
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Begin a smooth stage transition:
    /// - `hide_windows` swoop from desktop bounds into `from_card_screen`
    /// - `show_windows` swoop from `to_card_screen` onto desktop bounds
    pub fn start_transition(
        &mut self,
        hide_windows: &[isize],
        show_windows: &[isize],
        from_card_screen: RECT,
        to_card_screen: RECT,
        work_area: &RECT,
        tracked: &HashMap<isize, TrackedWindow>,
        sidebar_hwnd: HWND,
    ) {
        // If an earlier transition had pending restores, commit them immediately
        self.flush_pending_restores();
        self.thumbnails.clear();
        self.items.clear();
        self.restored_real_windows = false;
        self.linger_frames = 0;

        let ox = self.screen_origin_x;
        let oy = self.screen_origin_y;
        let to_local = |r: RECT| RECT {
            left: r.left - ox,
            top: r.top - oy,
            right: r.right - ox,
            bottom: r.bottom - oy,
        };

        // 1. Register thumbnails for outgoing windows BEFORE minimizing them
        for &hk in hide_windows {
            let hwnd = HWND(hk as *mut _);
            let Some(tw) = tracked.get(&hk) else {
                continue;
            };
            if !tw.is_minimized && self.thumbnails.ensure(hwnd) {
                let start = to_local(if tw.visual_rect.right > tw.visual_rect.left {
                    tw.visual_rect
                } else {
                    tw.desktop_target_rect(work_area)
                });
                let end = to_local(from_card_screen);
                let mut rs = SpringRect::new(start, 260.0, 32.0);
                rs.set_target(end);
                let mut os = Spring::new(255.0, 300.0, 34.0);
                os.set_target(0.0);

                self.thumbnails
                    .update(hk, start, Some(tw.frame_insets), 255, true);

                self.items.push(AnimatingWindow {
                    hwnd_key: hk,
                    insets: Some(tw.frame_insets),
                    rect_spring: rs,
                    opacity_spring: os,
                    is_incoming: false,
                });
            }
        }

        // 2. Register thumbnails for incoming windows
        self.pending_restore.clear();
        for &hk in show_windows {
            let hwnd = HWND(hk as *mut _);
            if let Some(tw) = tracked.get(&hk) {
                self.pending_restore.push((hk, tw.was_maximized));
                if self.thumbnails.ensure(hwnd) {
                    if let Some((sw, sh)) = self.thumbnails.source_size(hk, Some(tw.frame_insets)) {
                        if sw > 16 && sh > 16 {
                            let start = to_local(to_card_screen);
                            let end = to_local(tw.desktop_target_rect(work_area));
                            let mut rs = SpringRect::new(start, 260.0, 32.0);
                            rs.set_target(end);
                            let mut os = Spring::new(0.0, 300.0, 34.0);
                            os.set_target(255.0);

                            self.thumbnails
                                .update(hk, start, Some(tw.frame_insets), 0, true);

                            self.items.push(AnimatingWindow {
                                hwnd_key: hk,
                                insets: Some(tw.frame_insets),
                                rect_spring: rs,
                                opacity_spring: os,
                                is_incoming: true,
                            });
                        }
                    }
                }
            }
        }

        // Show overlay above desktop windows but keep sidebar topmost on top of it
        if !self.items.is_empty() {
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                let _ = InvalidateRect(self.hwnd, None, false);
                let _ = UpdateWindow(self.hwnd);
                let _ = SetWindowPos(
                    self.hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                let _ = SetWindowPos(
                    sidebar_hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            self.active = true;
        } else {
            self.active = false;
        }

        // Minimize outgoing windows cleanly with system animations suppressed
        for &hk in hide_windows {
            minimize_instant(HWND(hk as *mut _));
        }

        // If no incoming thumbnail animations were created (e.g. cold cache), restore immediately
        let has_incoming_anims = self.items.iter().any(|i| i.is_incoming);
        if !has_incoming_anims {
            self.flush_pending_restores();
        }
    }

    /// Advance transition springs by `dt`. Returns `true` while overlay remains active.
    pub fn step(&mut self, dt: f64) -> bool {
        if !self.active {
            return false;
        }

        let mut any_moving = false;
        let mut incoming_ready = true;

        for item in &mut self.items {
            let moving_rect = item.rect_spring.step(dt);
            let moving_op = item.opacity_spring.step(dt);
            if moving_rect || moving_op {
                any_moving = true;
            }
            if item.is_incoming && !item.rect_spring.left.is_settled() {
                let dist = (item.rect_spring.left.target - item.rect_spring.left.value).abs()
                    + (item.rect_spring.right.target - item.rect_spring.right.value).abs();
                if dist > 10.0 {
                    incoming_ready = false;
                }
            }

            let rect = item.rect_spring.current_rect();
            let opacity = item.opacity_spring.value.clamp(0.0, 255.0).round() as u8;
            self.thumbnails
                .update(item.hwnd_key, rect, item.insets, opacity, true);
        }

        if !self.restored_real_windows {
            if !any_moving || (incoming_ready && !self.pending_restore.is_empty()) {
                self.flush_pending_restores();
                self.linger_frames = 2;
                self.restored_real_windows = true;
            }
        } else if !any_moving {
            if self.linger_frames > 0 {
                self.linger_frames -= 1;
            } else {
                self.finish_and_hide();
                return false;
            }
        }

        true
    }

    pub fn flush_pending_restores(&mut self) {
        let to_restore = std::mem::take(&mut self.pending_restore);
        // Restore back-to-front so index 0 ends up in front
        for &(hk, was_max) in to_restore.iter().rev() {
            restore_instant(HWND(hk as *mut _), was_max);
        }
        if let Some(&(front_hk, _)) = to_restore.first() {
            unsafe {
                let _ = SetForegroundWindow(HWND(front_hk as *mut _));
            }
        }
    }

    fn finish_and_hide(&mut self) {
        self.thumbnails.clear();
        self.items.clear();
        self.active = false;
        self.restored_real_windows = false;
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

impl Drop for TransitionOverlay {
    fn drop(&mut self) {
        self.flush_pending_restores();
        self.thumbnails.clear();
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

/// Minimize a window without Windows' built-in taskbar shrink animation.
pub fn minimize_instant(hwnd: HWND) {
    unsafe {
        if !IsWindow(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return;
        }
        let disable: BOOL = true.into();
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &disable as *const BOOL as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
        let enable: BOOL = false.into();
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &enable as *const BOOL as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );
    }
}

/// Restore a window without Windows' built-in taskbar zoom animation.
pub fn restore_instant(hwnd: HWND, was_maximized: bool) {
    unsafe {
        if !IsWindow(hwnd).as_bool() {
            return;
        }
        let disable: BOOL = true.into();
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &disable as *const BOOL as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );
        if IsIconic(hwnd).as_bool() {
            let cmd = if was_maximized {
                SW_SHOWMAXIMIZED
            } else {
                SW_RESTORE
            };
            let _ = ShowWindow(hwnd, cmd);
        } else {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        let enable: BOOL = false.into();
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &enable as *const BOOL as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );
    }
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rc = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            let brush = HBRUSH(GetStockObject(BLACK_BRUSH).0);
            FillRect(hdc, &rc, brush);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
