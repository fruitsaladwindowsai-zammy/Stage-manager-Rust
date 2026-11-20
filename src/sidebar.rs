//! macOS-style floating Stage Manager sidebar:
//! - Auto-hides off-screen by default, activating smoothly at any point on the left/right side
//! - Pure floating rounded DWM thumbnail windows + overlapping bottom-left squircle app icons
//! - Infinite circular vertical carousel scrolling when hovering top/bottom endpoints or using mouse wheel

use crate::animation::Spring;
use crate::blur::{self, GlassTheme};
use crate::config::{Config, SidebarEdge, TriggerMode};
use crate::desktop::MonitorInfo;
use crate::layout::{
    self, AnimatedStageBadge, AnimatedStageFrame, StageStripMetrics, StageTargetLayout,
};
use crate::renderer::IconOverlayRenderer;
use crate::stage_manager::{Stage, StageId};
use crate::thumbnail::ThumbnailLink;
use crate::window_tracker::{FrameInsets, TrackedWindow};
use log::info;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, ClientToScreen, EndPaint, FillRect, GetStockObject, ValidateRect, BLACK_BRUSH,
    HBRUSH, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Custom window messages sent to `Sidebar::hwnd` (the controller window).
pub const WM_STAGE_CORNER_TRIGGER: u32 = WM_APP + 20;
pub const WM_STAGE_POINTER_MOVE: u32 = WM_APP + 21;
pub const WM_STAGE_POINTER_CLICK: u32 = WM_APP + 22;
pub const WM_STAGE_POINTER_WHEEL: u32 = WM_APP + 23;

struct PreviewSlot {
    hwnd: HWND,
    link: Option<ThumbnailLink>,
    visible: bool,
}

impl Drop for PreviewSlot {
    fn drop(&mut self) {
        self.link = None;
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

#[derive(Debug, Clone)]
struct StageAnimState {
    target: StageTargetLayout,
    index_spring: Spring,
    hover_spring: Spring,
    expand_spring: Spring,
    appear_spring: Spring,
}

pub struct Sidebar {
    /// Hidden controller HWND receiving timers, tray icon, hotkeys, WinEvents, and forwarded input.
    pub hwnd: HWND,
    pub blur_hwnd: HWND,
    pub icon_hwnd: HWND,
    pub corner_hwnd: HWND,
    pub config: Config,
    pub monitor: MonitorInfo,
    pub work_area: RECT,
    pub dpi_scale: f64,
    pub strip_width: i32,
    pub work_height: i32,
    pub revealed: bool,
    pub hovered_stage: Option<StageId>,
    reveal_spring: Spring,
    pointer_y_spring: Spring,
    pointer_focus_spring: Spring,
    scroll_pos: f64,
    scroll_vel: f64,
    target_scroll_vel: f64,
    metrics: StageStripMetrics,
    preview_slots: HashMap<isize, PreviewSlot>,
    preview_class: Vec<u16>,
    hinstance: HINSTANCE,
    stage_anims: HashMap<StageId, StageAnimState>,
    stage_order: Vec<StageId>,
    last_frames: Vec<AnimatedStageFrame>,
    icon_renderer: IconOverlayRenderer,
    insets_cache: HashMap<isize, FrameInsets>,
    source_size_cache: HashMap<isize, (i32, i32)>,
    exe_cache: HashMap<isize, String>,
    windows_visible_on_screen: bool,
}

impl Sidebar {
    pub fn new_for_monitor(config: Config, monitor: MonitorInfo) -> windows::core::Result<Self> {
        let hmod = unsafe { GetModuleHandleW(None)? };
        let hinstance = HINSTANCE::from(hmod);

        let ctrl_class = wide("StageManagerController");
        let blur_class = wide("StageManagerBlurBackdrop");
        let preview_class = wide("StageManagerPreviewCard");
        let icon_class = wide("StageManagerIconOverlay");
        let corner_class = wide("StageManagerHotCorner");

        unsafe {
            let register = |cls: &[u16], proc_fn: WNDPROC, cursor: PCWSTR| {
                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: proc_fn,
                    hInstance: hinstance,
                    hCursor: LoadCursorW(None, cursor).unwrap_or_default(),
                    lpszClassName: PCWSTR(cls.as_ptr()),
                    ..Default::default()
                };
                let _ = RegisterClassExW(&wc);
            };

            register(&ctrl_class, Some(controller_wnd_proc), IDC_ARROW);
            register(&blur_class, Some(backdrop_wnd_proc), IDC_ARROW);
            register(&preview_class, Some(preview_wnd_proc), IDC_HAND);
            register(&icon_class, Some(icon_overlay_wnd_proc), IDC_ARROW);
            register(&corner_class, Some(corner_wnd_proc), IDC_ARROW);
        }

        let work_area = monitor.work_area;
        let dpi_scale = monitor.dpi_scale.max(0.5);

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                PCWSTR(ctrl_class.as_ptr()),
                PCWSTR(wide("Stage Manager").as_ptr()),
                WS_POPUP,
                work_area.left,
                work_area.top,
                1,
                1,
                None,
                None,
                hinstance,
                None,
            )?
        };

        let strip_width = (config.sidebar_width as f64 * dpi_scale).round() as i32;
        let work_height = (work_area.bottom - work_area.top).max(240);

        let blur_pad = (10.0 * dpi_scale).round() as i32;
        let blur_x = hidden_blur_x(&config, &work_area, strip_width, dpi_scale);
        let blur_hwnd = unsafe {
            let bh = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                PCWSTR(blur_class.as_ptr()),
                PCWSTR(wide("Stage Manager Blur").as_ptr()),
                WS_POPUP,
                blur_x,
                work_area.top + blur_pad,
                strip_width,
                (work_height - 2 * blur_pad).max(160),
                None,
                None,
                hinstance,
                None,
            )?;
            SetWindowLongPtrW(bh, GWLP_USERDATA, hwnd.0 as isize);
            bh
        };
        blur::apply_macos_glass_blur(blur_hwnd, config.glass_theme);

        let icon_w = strip_width + (64.0 * dpi_scale).round() as i32;
        let icon_hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE,
                PCWSTR(icon_class.as_ptr()),
                PCWSTR(wide("Stage Manager Icons").as_ptr()),
                WS_POPUP,
                work_area.left,
                work_area.top,
                icon_w,
                work_height,
                None,
                None,
                hinstance,
                None,
            )?
        };

        let (cx, cy, cw, ch) = corner_trigger_rect(&config, &work_area, dpi_scale);
        let corner_hwnd = unsafe {
            let ch_wnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_NOACTIVATE,
                PCWSTR(corner_class.as_ptr()),
                PCWSTR(wide("Stage Manager Edge Trigger").as_ptr()),
                WS_POPUP,
                cx,
                cy,
                cw,
                ch,
                None,
                None,
                hinstance,
                None,
            )?;
            let _ = SetLayeredWindowAttributes(ch_wnd, COLORREF(0), 1, LWA_ALPHA);
            SetWindowLongPtrW(ch_wnd, GWLP_USERDATA, hwnd.0 as isize);
            ch_wnd
        };

        let revealed = !config.auto_hide;
        let reveal_val = if revealed { 1.0 } else { 0.0 };
        let icon_renderer = IconOverlayRenderer::new(dpi_scale)?;
        let metrics =
            layout::compute_strip_metrics(0, strip_width, work_height, dpi_scale, &config);

        info!(
            "macOS Stage Manager UI initialized [monitor={}] (strip={}x{} @ DPI {:.0}%, auto_hide={}, trigger={:?})",
            monitor.monitor_key,
            strip_width,
            work_height,
            dpi_scale * 100.0,
            config.auto_hide,
            config.trigger_mode
        );

        Ok(Self {
            hwnd,
            blur_hwnd,
            icon_hwnd,
            corner_hwnd,
            config,
            monitor,
            work_area,
            dpi_scale,
            strip_width,
            work_height,
            revealed,
            hovered_stage: None,
            reveal_spring: Spring::snappy(reveal_val),
            pointer_y_spring: Spring::new(work_height as f64 * 0.5, 360.0, 32.0),
            pointer_focus_spring: Spring::new(0.0, 300.0, 28.0),
            scroll_pos: 0.0,
            scroll_vel: 0.0,
            target_scroll_vel: 0.0,
            metrics,
            preview_slots: HashMap::new(),
            preview_class,
            hinstance,
            stage_anims: HashMap::new(),
            stage_order: Vec::new(),
            last_frames: Vec::new(),
            icon_renderer,
            insets_cache: HashMap::new(),
            source_size_cache: HashMap::new(),
            exe_cache: HashMap::new(),
            windows_visible_on_screen: false,
        })
    }

    pub fn show_initial(&mut self) {
        if self.revealed {
            self.reveal_spring.set_target(1.0);
            self.apply_frame();
        } else {
            self.update_corner_visibility();
        }
    }

    pub fn owns_hwnd(&self, hwnd: HWND) -> bool {
        hwnd == self.hwnd
            || hwnd == self.blur_hwnd
            || hwnd == self.icon_hwnd
            || hwnd == self.corner_hwnd
    }

    pub fn set_revealed(&mut self, revealed: bool) {
        self.revealed = revealed;
        self.reveal_spring
            .set_target(if revealed { 1.0 } else { 0.0 });
        if revealed {
            let mut pt = POINT::default();
            if unsafe { GetCursorPos(&mut pt).is_ok() } {
                let py = (pt.y - self.work_area.top).clamp(0, self.work_height) as f64;
                self.pointer_y_spring = Spring::new(py, 360.0, 32.0);
                self.pointer_focus_spring.set_target(1.0);
            }
        } else {
            self.hovered_stage = None;
            self.target_scroll_vel = 0.0;
            self.pointer_focus_spring.set_target(0.0);
            for anim in self.stage_anims.values_mut() {
                anim.hover_spring.set_target(0.0);
            }
        }
        self.update_corner_visibility();
    }

    pub fn set_glass_theme(&mut self, theme: GlassTheme) {
        self.config.glass_theme = theme;
        blur::apply_macos_glass_blur(self.blur_hwnd, theme);
    }

    pub fn set_blur_backdrop(&mut self, enabled: bool) {
        self.config.blur_backdrop = enabled;
        self.apply_frame();
    }

    pub fn update_monitor(&mut self, monitor: MonitorInfo) {
        self.monitor = monitor;
        self.work_area = monitor.work_area;
        self.dpi_scale = monitor.dpi_scale.max(0.5);
        self.update_geometry_for_config();
    }

    pub fn update_geometry_for_config(&mut self) {
        self.work_area = self.monitor.work_area;
        self.dpi_scale = self.monitor.dpi_scale.max(0.5);
        self.strip_width = (self.config.sidebar_width as f64 * self.dpi_scale).round() as i32;
        self.work_height = (self.work_area.bottom - self.work_area.top).max(240);
        self.metrics = layout::compute_strip_metrics(
            self.stage_order.len(),
            self.strip_width,
            self.work_height,
            self.dpi_scale,
            &self.config,
        );

        let (cx, cy, cw, ch) = corner_trigger_rect(&self.config, &self.work_area, self.dpi_scale);
        unsafe {
            let _ = SetWindowPos(
                self.corner_hwnd,
                HWND_TOPMOST,
                cx,
                cy,
                cw,
                ch,
                SWP_NOACTIVATE,
            );
        }
        self.update_corner_visibility();
        if self.revealed || self.windows_visible_on_screen {
            self.apply_frame();
        }
    }

    fn update_corner_visibility(&self) {
        unsafe {
            if self.config.auto_hide && !self.revealed {
                let _ = ShowWindow(self.corner_hwnd, SW_SHOWNOACTIVATE);
                let _ = SetWindowPos(
                    self.corner_hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            } else {
                let _ = ShowWindow(self.corner_hwnd, SW_HIDE);
            }
        }
    }

    /// Scroll the infinite stage carousel via mouse wheel.
    pub fn scroll_by(&mut self, wheel_delta: i16) {
        if !self.metrics.overflows {
            return;
        }
        let impulse = -(wheel_delta as f64 / 120.0) * 620.0 * self.dpi_scale;
        self.scroll_vel += impulse;
    }

    /// Update continuous vertical auto-scroll velocity when the cursor enters the top or bottom endpoint.
    /// Returns `true` if edge scrolling or residual momentum is active.
    pub fn update_edge_scroll(&mut self, pt: POINT) -> bool {
        if !self.revealed || !self.metrics.overflows || !self.is_cursor_inside(pt) {
            self.target_scroll_vel = 0.0;
            return self.scroll_vel.abs() > 0.8;
        }

        let zone = 76.0 * self.dpi_scale;
        let top = self.work_area.top as f64;
        let bottom = self.work_area.bottom as f64;
        let py = pt.y as f64;

        if py <= top + zone {
            // Top endpoint: scroll backward / bring top off-screen apps down into view indefinitely
            let p = ((top + zone - py) / zone).clamp(0.0, 1.0);
            self.target_scroll_vel = -(180.0 + 480.0 * p * p) * self.dpi_scale;
        } else if py >= bottom - zone {
            // Bottom endpoint: scroll forward / bring bottom off-screen apps up into view indefinitely
            let p = ((py - (bottom - zone)) / zone).clamp(0.0, 1.0);
            self.target_scroll_vel = (180.0 + 480.0 * p * p) * self.dpi_scale;
        } else {
            self.target_scroll_vel = 0.0;
        }

        self.target_scroll_vel.abs() > 0.1 || self.scroll_vel.abs() > 0.8
    }

    pub fn is_edge_scrolling(&self) -> bool {
        self.target_scroll_vel.abs() > 0.1 || self.scroll_vel.abs() > 0.8
    }

    /// Check if screen-space `pt` is inside the revealed sidebar interaction region,
    /// including any currently expanded grouped stage fan-out.
    /// Extends slightly past top/bottom endpoints so resting at the edge triggers scrolling cleanly.
    pub fn is_cursor_inside(&self, pt: POINT) -> bool {
        let margin = (40.0 * self.dpi_scale).round() as i32;
        let (x_min, x_max) = match self.config.sidebar_edge {
            SidebarEdge::Left => (
                self.work_area.left,
                self.work_area.left + self.strip_width + margin,
            ),
            SidebarEdge::Right => (
                self.work_area.right - self.strip_width - margin,
                self.work_area.right,
            ),
        };
        if pt.x >= x_min
            && pt.x <= x_max
            && pt.y >= self.work_area.top - 4
            && pt.y <= self.work_area.bottom + 14
        {
            return true;
        }

        let pad = (24.0 * self.dpi_scale).round() as i32;
        self.last_frames.iter().any(|f| {
            (f.previews.len() > 1 || f.badge.expand_t > 0.02)
                && pt.x >= f.hit_rect.left - pad
                && pt.x <= f.hit_rect.right + pad
                && pt.y >= f.hit_rect.top - pad
                && pt.y <= f.hit_rect.bottom + pad
        })
    }

    /// Synchronize stage list, icons, and target slot indices.
    pub fn sync_stages(&mut self, stages: &[&Stage], tracked: &HashMap<isize, TrackedWindow>) {
        let mut active_icon_keys = HashSet::new();

        for stage in stages {
            for &hk in &stage.windows {
                active_icon_keys.insert(hk);
                if let Some(tw) = tracked.get(&hk) {
                    self.insets_cache.insert(hk, tw.frame_insets);
                    self.exe_cache.insert(hk, tw.exe_name.to_lowercase());
                    if tw.visual_rect.right > tw.visual_rect.left
                        && tw.visual_rect.bottom > tw.visual_rect.top
                    {
                        self.source_size_cache.insert(
                            hk,
                            (
                                tw.visual_rect.right - tw.visual_rect.left,
                                tw.visual_rect.bottom - tw.visual_rect.top,
                            ),
                        );
                    }
                    self.icon_renderer.cache_icon_if_needed(hk, tw.icon);
                }
            }
        }
        self.icon_renderer.retain_icons(&active_icon_keys);
        self.preview_slots
            .retain(|k, _| active_icon_keys.contains(k));

        self.metrics = layout::compute_strip_metrics(
            stages.len(),
            self.strip_width,
            self.work_height,
            self.dpi_scale,
            &self.config,
        );
        if !self.metrics.overflows {
            self.scroll_pos = 0.0;
            self.scroll_vel = 0.0;
            self.target_scroll_vel = 0.0;
        }

        let exe_map = &self.exe_cache;
        let targets = layout::build_stage_targets(stages, &self.config, &|stage| {
            distinct_app_icons(&stage.windows, exe_map)
        });

        let keep_ids: HashSet<StageId> = targets.iter().map(|t| t.stage_id).collect();
        self.stage_anims.retain(|id, _| keep_ids.contains(id));
        self.stage_order = targets.iter().map(|t| t.stage_id).collect();

        for target in targets {
            let sid = target.stage_id;
            let idx_f = target.slot_index as f64;
            let should_expand = self.hovered_stage == Some(sid) && target.windows.len() > 1;
            if let Some(anim) = self.stage_anims.get_mut(&sid) {
                anim.index_spring.set_target(idx_f);
                anim.expand_spring
                    .set_target(if should_expand { 1.0 } else { 0.0 });
                anim.target = target;
            } else {
                let mut index_spring = Spring::reflow(idx_f);
                index_spring.set_target(idx_f);

                let mut appear_spring = Spring::snappy(0.0);
                appear_spring.set_target(1.0);

                let mut expand_spring = Spring::hover(0.0);
                expand_spring.set_target(if should_expand { 1.0 } else { 0.0 });

                self.stage_anims.insert(
                    sid,
                    StageAnimState {
                        target,
                        index_spring,
                        hover_spring: Spring::hover(0.0),
                        expand_spring,
                        appear_spring,
                    },
                );
            }
        }

        if self.revealed || self.windows_visible_on_screen {
            self.apply_frame();
        }
    }

    pub fn on_pointer_move(&mut self, screen_x: i32, screen_y: i32) -> bool {
        let rel_y = (screen_y - self.work_area.top).clamp(0, self.work_height) as f64;
        let ptr_moved = (self.pointer_y_spring.target - rel_y).abs() > 0.5
            || self.pointer_focus_spring.target < 0.99;
        self.pointer_y_spring.set_target(rel_y);
        self.pointer_focus_spring.set_target(1.0);

        let hit = layout::hit_test_stages(&self.last_frames, screen_x, screen_y);
        if hit != self.hovered_stage {
            self.hovered_stage = hit;
            for (&sid, anim) in &mut self.stage_anims {
                let is_hovered = hit == Some(sid);
                anim.hover_spring
                    .set_target(if is_hovered { 1.0 } else { 0.0 });
                anim.expand_spring
                    .set_target(if is_hovered && anim.target.windows.len() > 1 {
                        1.0
                    } else {
                        0.0
                    });
            }
            return true;
        }
        ptr_moved
    }

    pub fn on_pointer_clear_hover(&mut self) -> bool {
        let mut changed = self.pointer_focus_spring.target > 0.01;
        self.pointer_focus_spring.set_target(0.0);
        if self.hovered_stage.is_some() {
            self.hovered_stage = None;
            for anim in self.stage_anims.values_mut() {
                anim.hover_spring.set_target(0.0);
                anim.expand_spring.set_target(0.0);
            }
            changed = true;
        }
        changed
    }

    pub fn on_pointer_click(
        &self,
        screen_x: i32,
        screen_y: i32,
    ) -> Option<(StageId, Option<isize>)> {
        layout::hit_test_stage_or_window(&self.last_frames, screen_x, screen_y)
    }

    pub fn card_screen_rect(&self, stage_id: StageId) -> RECT {
        if let Some(f) = self.last_frames.iter().find(|f| f.stage_id == stage_id) {
            return f.hit_rect;
        }
        if let Some(anim) = self.stage_anims.get(&stage_id) {
            let top =
                layout::compute_slot_top(anim.index_spring.value, self.scroll_pos, &self.metrics)
                    .round() as i32;
            let ox = self.work_area.left;
            let oy = self.work_area.top;
            return RECT {
                left: ox + self.metrics.slot_left,
                top: oy + top,
                right: ox + self.metrics.slot_right,
                bottom: oy + top + self.metrics.slot_h,
            };
        }
        RECT {
            left: self.work_area.left + 14,
            top: self.work_area.top + self.work_height / 2 - 55,
            right: self.work_area.left + 170,
            bottom: self.work_area.top + self.work_height / 2 + 55,
        }
    }

    /// Advance all springs and infinite carousel scroll by `dt` seconds.
    pub fn step_animations(&mut self, dt: f64) -> bool {
        let mut active = false;

        if self.reveal_spring.step_normalized(dt) {
            active = true;
        }
        if self.pointer_y_spring.step(dt) {
            active = true;
        }
        if self.pointer_focus_spring.step_normalized(dt) {
            active = true;
        }

        if self.metrics.overflows {
            let clamped_dt = dt.clamp(0.0001, 0.05);
            let blend = 1.0 - (-12.0 * clamped_dt).exp();
            self.scroll_vel += (self.target_scroll_vel - self.scroll_vel) * blend;

            if self.target_scroll_vel.abs() < 0.1 && self.scroll_vel.abs() < 1.2 {
                self.scroll_vel = 0.0;
            } else {
                self.scroll_pos += self.scroll_vel * clamped_dt;
                if self.metrics.cycle_h > 0.0 {
                    self.scroll_pos = self.scroll_pos.rem_euclid(self.metrics.cycle_h);
                }
                active = true;
            }
        }

        for anim in self.stage_anims.values_mut() {
            if anim.index_spring.step_normalized(dt) {
                active = true;
            }
            if anim.hover_spring.step_normalized(dt) {
                active = true;
            }
            if anim.expand_spring.step_normalized(dt) {
                active = true;
            }
            if anim.appear_spring.step_normalized(dt) {
                active = true;
            }
        }

        if self.revealed || self.reveal_spring.value > 0.005 {
            self.apply_frame();
        } else if self.windows_visible_on_screen {
            self.hide_all_visuals();
        }

        active
    }

    fn hide_all_visuals(&mut self) {
        self.windows_visible_on_screen = false;
        unsafe {
            let _ = ShowWindow(self.blur_hwnd, SW_HIDE);
            let _ = ShowWindow(self.icon_hwnd, SW_HIDE);
            for slot in self.preview_slots.values_mut() {
                if slot.visible {
                    let _ = ShowWindow(slot.hwnd, SW_HIDE);
                    slot.visible = false;
                }
            }
        }
        self.update_corner_visibility();
    }

    fn apply_frame(&mut self) {
        let rev = self.reveal_spring.value.clamp(-0.05, 1.05);
        if rev <= 0.005 && !self.revealed {
            if self.windows_visible_on_screen {
                self.hide_all_visuals();
            }
            return;
        }
        self.windows_visible_on_screen = true;
        let dir_sign = match self.config.sidebar_edge {
            SidebarEdge::Left => 1.0,
            SidebarEdge::Right => -1.0,
        };

        let travel = (self.strip_width as f64 + 32.0 * self.dpi_scale) * (1.0 - rev);
        let slide_offset_x = (-dir_sign * travel).round() as i32;

        let mut weighted_frames: Vec<(f64, AnimatedStageFrame)> =
            Vec::with_capacity(self.stage_order.len());
        let mut total_previews = 0usize;

        let base_w = (self.metrics.slot_right - self.metrics.slot_left) as f64;
        let base_h = self.metrics.slot_h as f64;
        let strip_origin_x = self.work_area.left as f64
            + match self.config.sidebar_edge {
                SidebarEdge::Left => 0.0,
                SidebarEdge::Right => {
                    (self.work_area.right - self.work_area.left - self.strip_width) as f64
                }
            };
        let p_focus = self.pointer_focus_spring.value.clamp(0.0, 1.0);
        let curve_radius = 175.0 * self.dpi_scale;

        for &sid in &self.stage_order {
            let Some(anim) = self.stage_anims.get(&sid) else {
                continue;
            };
            let t = &anim.target;
            let rel_top =
                layout::compute_slot_top(anim.index_spring.value, self.scroll_pos, &self.metrics);

            // Quick reject completely off-screen stages before creating DWM previews
            if rel_top + base_h < -28.0 || rel_top > self.work_height as f64 + 28.0 {
                continue;
            }

            let hover_t = anim.hover_spring.value;
            let expand_t = anim.expand_spring.value;
            let appear_t = anim.appear_spring.value;

            // macOS curved ribbon: active app under pointer pops outward, others bend inward
            let base_cy = rel_top + base_h * 0.5;
            let dy = base_cy - self.pointer_y_spring.value;
            let norm = dy / curve_radius;
            let wave = (-norm * norm * 1.15).exp();
            let active_t = ((0.65 * wave + 0.35 * hover_t) * p_focus + hover_t * (1.0 - p_focus))
                .clamp(0.0, 1.0);

            let bend_scale = 1.0 + p_focus * (0.14 * active_t - 0.14 * (1.0 - active_t));
            let scale = (0.90 + 0.10 * appear_t) * bend_scale;
            let curve_x = dir_sign
                * (15.0 * active_t - 14.0 * p_focus * (1.0 - active_t).powf(1.25))
                * self.dpi_scale;
            let neighbor_push =
                (dy / curve_radius) * (-norm * norm * 0.9).exp() * 14.0 * self.dpi_scale * p_focus;

            let cx = strip_origin_x
                + (self.metrics.slot_left + self.metrics.slot_right) as f64 * 0.5
                + curve_x
                + slide_offset_x as f64;
            let cy = self.work_area.top as f64 + base_cy + neighbor_push;

            let nw = (base_w * scale).round() as i32;
            let nh = (base_h * scale).round() as i32;
            let slot_screen = RECT {
                left: (cx - nw as f64 * 0.5).round() as i32,
                top: (cy - nh as f64 * 0.5).round() as i32,
                right: (cx + nw as f64 * 0.5).round() as i32,
                bottom: (cy + nh as f64 * 0.5).round() as i32,
            };

            let sizes = &self.source_size_cache;
            let previews = layout::layout_stage_cascade(
                &t.windows,
                &slot_screen,
                &self.work_area,
                self.dpi_scale,
                active_t,
                expand_t,
                self.config.sidebar_edge,
                &|hk| sizes.get(&hk).copied(),
            );
            total_previews += previews.len();

            let front_preview_rect = previews.first().map(|p| p.rect).unwrap_or(slot_screen);

            let mut union_left = slot_screen.left;
            let mut union_top = slot_screen.top;
            let mut union_right = slot_screen.right;
            let mut union_bottom = slot_screen.bottom;
            for p in &previews {
                union_left = union_left.min(p.rect.left);
                union_top = union_top.min(p.rect.top);
                union_right = union_right.max(p.rect.right);
                union_bottom = union_bottom.max(p.rect.bottom);
            }
            let hit_pad = (10.0 * self.dpi_scale).round() as i32;
            let hit_rect = RECT {
                left: union_left - hit_pad,
                top: union_top.max(self.work_area.top),
                right: union_right + hit_pad,
                bottom: (union_bottom + (12.0 * self.dpi_scale).round() as i32)
                    .min(self.work_area.bottom),
            };
            let expanded_previews: Vec<(isize, RECT)> =
                previews.iter().map(|p| (p.hwnd_key, p.rect)).collect();

            weighted_frames.push((
                active_t + expand_t,
                AnimatedStageFrame {
                    stage_id: sid,
                    hit_rect,
                    previews,
                    badge: AnimatedStageBadge {
                        icon_hwnds: t.icon_hwnds.clone(),
                        front_preview_rect,
                        window_count: t.window_count,
                        hover_t: active_t,
                        expand_t,
                        appear_t,
                        expanded_previews,
                    },
                },
            ));
        }

        // Order highest-focus card first so DeferWindowPos keeps the active card in front
        weighted_frames.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let frames: Vec<AnimatedStageFrame> = weighted_frames.into_iter().map(|(_, f)| f).collect();

        struct SlotDraw {
            dest_screen: RECT,
            clip_left_frac: f64,
            clip_right_frac: f64,
            clip_top_frac: f64,
            clip_bot_frac: f64,
            opacity: u8,
            hwnd_key: isize,
        }

        let mut active_slots: Vec<SlotDraw> = Vec::with_capacity(total_previews);
        let mut active_keys: HashSet<isize> = HashSet::with_capacity(total_previews);
        for frame in &frames {
            for p in &frame.previews {
                // Reject cards entirely outside this monitor's work area to prevent multi-monitor bleed
                if p.rect.bottom <= self.work_area.top
                    || p.rect.top >= self.work_area.bottom
                    || p.rect.right <= self.work_area.left
                    || p.rect.left >= self.work_area.right
                {
                    continue;
                }

                let full_w = (p.rect.right - p.rect.left).max(1) as f64;
                let full_h = (p.rect.bottom - p.rect.top).max(1) as f64;

                let clip_left_frac =
                    ((self.work_area.left - p.rect.left).max(0) as f64 / full_w).clamp(0.0, 0.98);
                let clip_right_frac =
                    ((p.rect.right - self.work_area.right).max(0) as f64 / full_w).clamp(0.0, 0.98);
                let clip_top_frac =
                    ((self.work_area.top - p.rect.top).max(0) as f64 / full_h).clamp(0.0, 0.98);
                let clip_bot_frac = ((p.rect.bottom - self.work_area.bottom).max(0) as f64
                    / full_h)
                    .clamp(0.0, 0.98);

                let clipped = RECT {
                    left: p.rect.left.max(self.work_area.left),
                    top: p.rect.top.max(self.work_area.top),
                    right: p.rect.right.min(self.work_area.right),
                    bottom: p.rect.bottom.min(self.work_area.bottom),
                };
                if clipped.bottom - clipped.top < 12 || clipped.right - clipped.left < 12 {
                    continue;
                }

                if !self.preview_slots.contains_key(&p.hwnd_key) {
                    if let Ok(slot) = self.create_preview_slot(p.hwnd_key) {
                        self.preview_slots.insert(p.hwnd_key, slot);
                    }
                }
                if active_keys.insert(p.hwnd_key) {
                    active_slots.push(SlotDraw {
                        dest_screen: clipped,
                        clip_left_frac,
                        clip_right_frac,
                        clip_top_frac,
                        clip_bot_frac,
                        opacity: p.opacity,
                        hwnd_key: p.hwnd_key,
                    });
                }
            }
        }

        for draw in &active_slots {
            let w = (draw.dest_screen.right - draw.dest_screen.left).max(1);
            let h = (draw.dest_screen.bottom - draw.dest_screen.top).max(1);
            if let Some(slot) = self.preview_slots.get(&draw.hwnd_key) {
                if let Some(ref link) = slot.link {
                    if let Ok((sw, sh)) = link.source_size() {
                        if sw > 16 && sh > 16 {
                            self.source_size_cache.insert(draw.hwnd_key, (sw, sh));
                        }
                    }
                    let _ = link.update_clipped(
                        RECT {
                            left: 0,
                            top: 0,
                            right: w,
                            bottom: h,
                        },
                        self.insets_cache.get(&draw.hwnd_key).copied(),
                        draw.clip_left_frac,
                        draw.clip_right_frac,
                        draw.clip_top_frac,
                        draw.clip_bot_frac,
                        draw.opacity,
                        true,
                    );
                }
            }
        }

        let base_icon_w = self.strip_width + (64.0 * self.dpi_scale).round() as i32;
        let (mut icon_min_x, mut icon_max_x) = match self.config.sidebar_edge {
            SidebarEdge::Left => (self.work_area.left, self.work_area.left + base_icon_w),
            SidebarEdge::Right => (self.work_area.right - base_icon_w, self.work_area.right),
        };
        let badge_margin = (44.0 * self.dpi_scale).round() as i32;
        for f in &frames {
            let fp = &f.badge.front_preview_rect;
            if fp.bottom <= self.work_area.top + 16
                || fp.top >= self.work_area.bottom - 16
                || fp.right <= self.work_area.left
                || fp.left >= self.work_area.right
            {
                continue;
            }
            for (_, r) in &f.badge.expanded_previews {
                icon_min_x = icon_min_x.min(r.left - badge_margin);
                icon_max_x = icon_max_x.max(r.right + badge_margin);
            }
        }
        icon_min_x = icon_min_x.max(self.work_area.left);
        icon_max_x = icon_max_x.min(self.work_area.right);
        let icon_x = icon_min_x;
        let icon_w = (icon_max_x - icon_min_x).max(0);
        let icon_y = self.work_area.top;
        let show_icons = icon_w >= 12 && rev > 0.02;

        if show_icons {
            let local_badges: Vec<AnimatedStageBadge> = frames
                .iter()
                .filter(|f| {
                    let fp = &f.badge.front_preview_rect;
                    fp.bottom > self.work_area.top + 16
                        && fp.top < self.work_area.bottom - 16
                        && fp.right > self.work_area.left
                        && fp.left < self.work_area.right
                })
                .map(|f| {
                    let mut b = f.badge.clone();
                    b.front_preview_rect = RECT {
                        left: b.front_preview_rect.left - icon_x,
                        top: b.front_preview_rect.top - icon_y,
                        right: b.front_preview_rect.right - icon_x,
                        bottom: b.front_preview_rect.bottom - icon_y,
                    };
                    for (_, rect) in &mut b.expanded_previews {
                        *rect = RECT {
                            left: rect.left - icon_x,
                            top: rect.top - icon_y,
                            right: rect.right - icon_x,
                            bottom: rect.bottom - icon_y,
                        };
                    }
                    b
                })
                .collect();

            unsafe {
                if !IsWindowVisible(self.icon_hwnd).as_bool() {
                    let _ = ShowWindow(self.icon_hwnd, SW_SHOWNOACTIVATE);
                }
            }
            self.icon_renderer.render_layered(
                self.icon_hwnd,
                icon_x,
                icon_y,
                icon_w,
                self.work_height,
                self.dpi_scale,
                self.config.sidebar_edge,
                &local_badges,
            );
        } else {
            unsafe {
                let _ = ShowWindow(self.icon_hwnd, SW_HIDE);
            }
        }

        let blur_pad = (10.0 * self.dpi_scale).round() as i32;
        let raw_blur_x = match self.config.sidebar_edge {
            SidebarEdge::Left => self.work_area.left + blur_pad + slide_offset_x,
            SidebarEdge::Right => {
                self.work_area.right - self.strip_width - blur_pad + slide_offset_x
            }
        };
        let blur_left = raw_blur_x.max(self.work_area.left);
        let blur_right = (raw_blur_x + self.strip_width).min(self.work_area.right);
        let blur_w = (blur_right - blur_left).max(0);
        let blur_h = (self.work_height - 2 * blur_pad).max(160);
        let show_blur = self.config.blur_backdrop && blur_w >= 12 && rev > 0.02;

        let unused_count = self.preview_slots.len().saturating_sub(active_keys.len());
        let total_defer = 2 + active_slots.len() + unused_count;

        unsafe {
            if let Ok(mut hdwp) = BeginDeferWindowPos(total_defer as i32) {
                let mut insert_after = HWND_TOPMOST;

                let icon_flags = if show_icons {
                    SWP_NOACTIVATE | SWP_SHOWWINDOW
                } else {
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_HIDEWINDOW
                };
                if let Ok(next) = DeferWindowPos(
                    hdwp,
                    self.icon_hwnd,
                    insert_after,
                    icon_x,
                    icon_y,
                    icon_w.max(1),
                    self.work_height,
                    icon_flags,
                ) {
                    hdwp = next;
                    insert_after = self.icon_hwnd;
                }

                for draw in &active_slots {
                    if let Some(slot) = self.preview_slots.get_mut(&draw.hwnd_key) {
                        let rect = draw.dest_screen;
                        let w = (rect.right - rect.left).max(1);
                        let h = (rect.bottom - rect.top).max(1);
                        if let Ok(next) = DeferWindowPos(
                            hdwp,
                            slot.hwnd,
                            insert_after,
                            rect.left,
                            rect.top,
                            w,
                            h,
                            SWP_NOACTIVATE | SWP_SHOWWINDOW,
                        ) {
                            hdwp = next;
                            insert_after = slot.hwnd;
                            slot.visible = true;
                        }
                    }
                }

                for (&hk, slot) in &mut self.preview_slots {
                    if !active_keys.contains(&hk) && slot.visible {
                        if let Ok(next) = DeferWindowPos(
                            hdwp,
                            slot.hwnd,
                            HWND_BOTTOM,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE
                                | SWP_NOSIZE
                                | SWP_NOZORDER
                                | SWP_NOACTIVATE
                                | SWP_HIDEWINDOW,
                        ) {
                            hdwp = next;
                        }
                        slot.visible = false;
                    }
                }

                let blur_flags = if show_blur {
                    SWP_NOACTIVATE | SWP_SHOWWINDOW
                } else {
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_HIDEWINDOW
                };
                if let Ok(next) = DeferWindowPos(
                    hdwp,
                    self.blur_hwnd,
                    insert_after,
                    blur_left,
                    self.work_area.top + blur_pad,
                    blur_w.max(1),
                    blur_h,
                    blur_flags,
                ) {
                    hdwp = next;
                }

                let _ = EndDeferWindowPos(hdwp);
            }
        }

        self.last_frames = frames;
    }

    fn create_preview_slot(&self, src_hwnd_key: isize) -> windows::core::Result<PreviewSlot> {
        unsafe {
            let h = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                PCWSTR(self.preview_class.as_ptr()),
                PCWSTR(wide("Stage Preview").as_ptr()),
                WS_POPUP,
                self.work_area.left,
                self.work_area.top,
                160,
                100,
                None,
                None,
                self.hinstance,
                None,
            )?;
            SetWindowLongPtrW(h, GWLP_USERDATA, self.hwnd.0 as isize);
            blur::apply_rounded_preview_style(h);
            let link = ThumbnailLink::new(h, HWND(src_hwnd_key as *mut _)).ok();
            Ok(PreviewSlot {
                hwnd: h,
                link,
                visible: false,
            })
        }
    }
}

impl Drop for Sidebar {
    fn drop(&mut self) {
        self.preview_slots.clear();
        unsafe {
            let _ = DestroyWindow(self.corner_hwnd);
            let _ = DestroyWindow(self.icon_hwnd);
            let _ = DestroyWindow(self.blur_hwnd);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn distinct_app_icons(windows: &[isize], exe_map: &HashMap<isize, String>) -> Vec<isize> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for &hk in windows {
        let key = exe_map
            .get(&hk)
            .filter(|e| !e.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("hwnd:{hk}"));
        if seen.insert(key) {
            out.push(hk);
            if out.len() >= 3 {
                break;
            }
        }
    }
    if out.is_empty() {
        if let Some(&first) = windows.first() {
            out.push(first);
        }
    }
    out
}

fn hidden_blur_x(config: &Config, work_area: &RECT, strip_w: i32, _dpi_scale: f64) -> i32 {
    match config.sidebar_edge {
        SidebarEdge::Left => work_area.left,
        SidebarEdge::Right => work_area.right - strip_w,
    }
}

fn corner_trigger_rect(config: &Config, work_area: &RECT, dpi_scale: f64) -> (i32, i32, i32, i32) {
    let corner_sz = (18.0 * dpi_scale).round() as i32;
    let strip_w = (5.0 * dpi_scale).round().max(4.0) as i32;
    let mon_h = (work_area.bottom - work_area.top).max(120);

    match (config.sidebar_edge, config.trigger_mode) {
        (SidebarEdge::Left, TriggerMode::TopCorner) => {
            (work_area.left, work_area.top, corner_sz, corner_sz)
        }
        (SidebarEdge::Right, TriggerMode::TopCorner) => (
            work_area.right - corner_sz,
            work_area.top,
            corner_sz,
            corner_sz,
        ),
        (SidebarEdge::Left, TriggerMode::BottomCorner) => (
            work_area.left,
            work_area.bottom - corner_sz,
            corner_sz,
            corner_sz,
        ),
        (SidebarEdge::Right, TriggerMode::BottomCorner) => (
            work_area.right - corner_sz,
            work_area.bottom - corner_sz,
            corner_sz,
            corner_sz,
        ),
        (SidebarEdge::Left, TriggerMode::FullEdge) => {
            (work_area.left, work_area.top, strip_w, mon_h)
        }
        (SidebarEdge::Right, TriggerMode::FullEdge) => {
            (work_area.right - strip_w, work_area.top, strip_w, mon_h)
        }
    }
}

unsafe fn forward_pointer(hwnd: HWND, msg_id: u32, wparam: WPARAM, lparam: LPARAM) {
    let ctrl_raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if ctrl_raw == 0 {
        return;
    }
    let ctrl = HWND(ctrl_raw as *mut _);
    if msg_id == WM_STAGE_POINTER_WHEEL {
        let _ = PostMessageW(ctrl, msg_id, wparam, lparam);
        return;
    }
    let x = (lparam.0 & 0xFFFF) as u16 as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as u16 as i16 as i32;
    let mut pt = POINT { x, y };
    let _ = ClientToScreen(hwnd, &mut pt);
    let packed = ((pt.y as i16 as u16 as usize) << 16) | (pt.x as i16 as u16 as usize);
    let _ = PostMessageW(ctrl, msg_id, wparam, LPARAM(packed as isize));
}

unsafe extern "system" fn controller_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe extern "system" fn backdrop_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
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
        WM_MOUSEMOVE => {
            forward_pointer(hwnd, WM_STAGE_POINTER_MOVE, wparam, lparam);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            forward_pointer(hwnd, WM_STAGE_POINTER_CLICK, wparam, lparam);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            forward_pointer(hwnd, WM_STAGE_POINTER_WHEEL, wparam, lparam);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn preview_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let _ = ValidateRect(hwnd, None);
            LRESULT(0)
        }
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_MOUSEMOVE => {
            forward_pointer(hwnd, WM_STAGE_POINTER_MOVE, wparam, lparam);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            forward_pointer(hwnd, WM_STAGE_POINTER_CLICK, wparam, lparam);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            forward_pointer(hwnd, WM_STAGE_POINTER_WHEEL, wparam, lparam);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn icon_overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn corner_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_MOUSEMOVE {
        let ctrl_raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if ctrl_raw != 0 {
            let _ = PostMessageW(
                HWND(ctrl_raw as *mut _),
                WM_STAGE_CORNER_TRIGGER,
                WPARAM(0),
                LPARAM(0),
            );
        }
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
