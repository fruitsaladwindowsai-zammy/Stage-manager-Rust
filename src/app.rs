//! Main application coordinator and Win32 message loop.
//! Supports:
//! - Independent Stage Manager sidebars per connected monitor
//! - Isolated workspace stage state per Windows Virtual Desktop
//! - System tray controls + Windows startup toggle

use crate::animation::{FrameClock, VsyncTicker, WM_STAGE_VSYNC};
use crate::blur::GlassTheme;
use crate::config::{Config, GroupingMode, SidebarEdge, TriggerMode};
use crate::desktop::{
    enumerate_monitors, is_run_at_startup_enabled, monitor_for_point, monitor_for_window,
    set_run_at_startup, DesktopId, MonitorInfo,
};
use crate::hotkeys;
use crate::sidebar::{
    Sidebar, WM_STAGE_CORNER_TRIGGER, WM_STAGE_POINTER_CLICK, WM_STAGE_POINTER_MOVE,
    WM_STAGE_POINTER_WHEEL,
};
use crate::stage_manager::{StageManager, SwitchPlan};
use crate::transition::{restore_instant, TransitionOverlay};
use crate::tray::{
    TrayIcon, MENU_AUTO_HIDE, MENU_BLUR_BACKDROP, MENU_EDGE_LEFT, MENU_EDGE_RIGHT, MENU_EXIT,
    MENU_GROUP_BY_APP, MENU_GROUP_SINGLE, MENU_INCLUDE_ACTIVE, MENU_RESTORE_ALL,
    MENU_RUN_AT_STARTUP, MENU_THEME_DARK, MENU_THEME_LIGHT, MENU_TOGGLE_SIDEBAR,
    MENU_TRIGGER_BOTTOM_CORNER, MENU_TRIGGER_FULL_EDGE, MENU_TRIGGER_TOP_CORNER, WM_TRAY_ICON,
};
use crate::window_tracker::{TrackedWindow, WindowTracker, WM_STAGE_WIN_EVENT};
use log::info;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::*;

const SAFETY_SYNC_TIMER_ID: usize = 1;
const DEBOUNCE_SYNC_TIMER_ID: usize = 2;
const HOVER_WATCH_TIMER_ID: usize = 3;
const WM_STAGE_DISPLAY_CHANGE: u32 = WM_APP + 30;
const WM_STAGE_TRAY_QUEUED: u32 = WM_APP + 31;
const WM_STAGE_TASKBAR_CREATED: u32 = WM_APP + 32;

static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);

pub struct App {
    app_hwnd: HWND,
    config: Config,
    tracker: WindowTracker,
    workspaces: HashMap<(DesktopId, isize), StageManager>,
    active_desktop_id: DesktopId,
    monitors: HashMap<isize, MonitorInfo>,
    primary_monitor: isize,
    sidebars: HashMap<isize, Sidebar>,
    transition: TransitionOverlay,
    vsync: VsyncTicker,
    clock: FrameClock,
    tray: Option<TrayIcon>,
    taskbar_created_msg: u32,
    tracked_map: HashMap<isize, TrackedWindow>,
    outside_since: HashMap<isize, Instant>,
    hover_watch_active: bool,
}

impl App {
    pub fn new() -> windows::core::Result<Self> {
        let config = Config::default();
        let app_hwnd = create_app_host_window()?;

        let monitor_list = enumerate_monitors();
        let mut monitors = HashMap::with_capacity(monitor_list.len());
        let mut sidebars = HashMap::with_capacity(monitor_list.len());
        let mut primary_monitor = monitor_list.first().map(|m| m.monitor_key).unwrap_or(0);

        for m in monitor_list {
            if m.is_primary {
                primary_monitor = m.monitor_key;
            }
            let sb = Sidebar::new_for_monitor(config.clone(), m)?;
            sidebars.insert(m.monitor_key, sb);
            monitors.insert(m.monitor_key, m);
        }

        let transition = TransitionOverlay::new()?;
        let vsync = VsyncTicker::new(app_hwnd);

        let mut tracker = WindowTracker::new();
        let initial_windows = tracker.sync_all();
        tracker.install_hooks(app_hwnd);

        let tray = TrayIcon::new(app_hwnd);
        let taskbar_msg_w = wide("TaskbarCreated");
        let taskbar_created_msg = unsafe { RegisterWindowMessageW(PCWSTR(taskbar_msg_w.as_ptr())) };
        TASKBAR_CREATED_MSG.store(taskbar_created_msg, Ordering::Release);
        hotkeys::register_hotkeys(app_hwnd, config.toggle_hotkey_vk);

        unsafe {
            SetTimer(app_hwnd, SAFETY_SYNC_TIMER_ID, 1500, None);
        }

        let mut app = Self {
            app_hwnd,
            config,
            tracker,
            workspaces: HashMap::new(),
            active_desktop_id: [0u8; 16],
            monitors,
            primary_monitor,
            sidebars,
            transition,
            vsync,
            clock: FrameClock::new(),
            tray,
            taskbar_created_msg,
            tracked_map: HashMap::new(),
            outside_since: HashMap::new(),
            hover_watch_active: false,
        };

        app.apply_tracked_windows(initial_windows);

        info!(
            "Stage Manager ready — {} monitor(s), auto_hide={}, corner={:?}",
            app.sidebars.len(),
            app.config.auto_hide,
            app.config.trigger_mode
        );

        Ok(app)
    }

    pub fn run(&mut self) {
        self.sync_all_sidebars_from_workspaces(true);
        for sb in self.sidebars.values_mut() {
            sb.show_initial();
        }
        if self.sidebars.values().any(|s| s.revealed) {
            self.ensure_hover_watch();
            self.wake_vsync();
        }

        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                self.handle_message(&msg);
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        hotkeys::unregister_hotkeys(self.app_hwnd);
        unsafe {
            let _ = KillTimer(self.app_hwnd, SAFETY_SYNC_TIMER_ID);
            let _ = KillTimer(self.app_hwnd, DEBOUNCE_SYNC_TIMER_ID);
            let _ = KillTimer(self.app_hwnd, HOVER_WATCH_TIMER_ID);
            let _ = DestroyWindow(self.app_hwnd);
        }
        info!("Stage Manager exiting cleanly");
    }

    fn wake_vsync(&self) {
        self.vsync.set_active(true);
    }

    fn ensure_hover_watch(&mut self) {
        if self.config.auto_hide && !self.hover_watch_active {
            self.hover_watch_active = true;
            unsafe {
                SetTimer(self.app_hwnd, HOVER_WATCH_TIMER_ID, 70, None);
            }
        }
    }

    fn stop_hover_watch_if_idle(&mut self) {
        let any_revealed = self.sidebars.values().any(|s| s.revealed);
        if !any_revealed && self.hover_watch_active {
            self.hover_watch_active = false;
            self.outside_since.clear();
            unsafe {
                let _ = KillTimer(self.app_hwnd, HOVER_WATCH_TIMER_ID);
            }
        }
    }

    fn monitor_for_sidebar_hwnd(&self, hwnd: HWND) -> Option<isize> {
        self.sidebars
            .iter()
            .find(|(_, sb)| sb.hwnd == hwnd)
            .map(|(&mk, _)| mk)
    }

    fn target_monitor_for_input(&self) -> isize {
        let mut pt = POINT::default();
        if unsafe { GetCursorPos(&mut pt).is_ok() } {
            let mk = monitor_for_point(pt);
            if self.sidebars.contains_key(&mk) {
                return mk;
            }
        }
        let fg = unsafe { GetForegroundWindow() };
        if !fg.0.is_null() {
            let mk = monitor_for_window(fg);
            if self.sidebars.contains_key(&mk) {
                return mk;
            }
        }
        if self.sidebars.contains_key(&self.primary_monitor) {
            return self.primary_monitor;
        }
        *self.sidebars.keys().next().unwrap_or(&0)
    }

    fn handle_message(&mut self, msg: &MSG) {
        match msg.message {
            WM_STAGE_VSYNC => self.on_vsync_tick(),

            WM_STAGE_WIN_EVENT => {
                let event = msg.wParam.0 as u32;
                let hwnd = HWND(msg.lParam.0 as *mut _);
                self.on_win_event(event, hwnd);
            }

            WM_STAGE_DISPLAY_CHANGE => self.sync_monitors(),

            WM_STAGE_CORNER_TRIGGER => {
                if let Some(mk) = self.monitor_for_sidebar_hwnd(msg.hwnd) {
                    if self.config.auto_hide
                        && self.sidebars.get(&mk).is_some_and(|sb| !sb.revealed)
                    {
                        self.full_window_sync();
                        if let Some(sb) = self.sidebars.get_mut(&mk) {
                            sb.set_revealed(true);
                        }
                        self.outside_since.remove(&mk);
                        self.ensure_hover_watch();
                        self.wake_vsync();
                    }
                }
            }

            WM_STAGE_POINTER_MOVE => {
                if let Some(mk) = self.monitor_for_sidebar_hwnd(msg.hwnd) {
                    let (sx, sy) = unpack_lparam_coords(msg.lParam.0);
                    self.outside_since.remove(&mk);
                    if let Some(sb) = self.sidebars.get_mut(&mk) {
                        let hover_changed = sb.on_pointer_move(sx, sy);
                        let scrolling = sb.update_edge_scroll(POINT { x: sx, y: sy });
                        if hover_changed || scrolling {
                            self.wake_vsync();
                        }
                    }
                }
            }

            WM_STAGE_POINTER_CLICK => {
                if let Some(mk) = self.monitor_for_sidebar_hwnd(msg.hwnd) {
                    let (sx, sy) = unpack_lparam_coords(msg.lParam.0);
                    let hit = self
                        .sidebars
                        .get(&mk)
                        .and_then(|sb| sb.on_pointer_click(sx, sy));
                    if let Some((stage_id, focus_hwnd)) = hit {
                        let shift_down = unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0;
                        if shift_down {
                            self.merge_stage(mk, stage_id);
                        } else {
                            let desktop_id = self.active_desktop_id;
                            let plan = self
                                .workspaces
                                .get_mut(&(desktop_id, mk))
                                .and_then(|sm| sm.switch_to_window(stage_id, focus_hwnd));
                            if let Some(plan) = plan {
                                self.execute_switch_plan(mk, plan);
                                if self.config.auto_hide {
                                    if let Some(sb) = self.sidebars.get_mut(&mk) {
                                        sb.set_revealed(false);
                                    }
                                    self.stop_hover_watch_if_idle();
                                    self.wake_vsync();
                                }
                            }
                        }
                    }
                }
            }

            WM_STAGE_POINTER_WHEEL => {
                if let Some(mk) = self.monitor_for_sidebar_hwnd(msg.hwnd) {
                    let delta = ((msg.wParam.0 >> 16) & 0xFFFF) as u16 as i16;
                    if let Some(sb) = self.sidebars.get_mut(&mk) {
                        sb.scroll_by(delta);
                        self.wake_vsync();
                    }
                }
            }

            WM_TIMER => match msg.wParam.0 {
                SAFETY_SYNC_TIMER_ID => {
                    if !self.transition.is_active() {
                        self.full_window_sync();
                    }
                }
                DEBOUNCE_SYNC_TIMER_ID => {
                    unsafe {
                        let _ = KillTimer(self.app_hwnd, DEBOUNCE_SYNC_TIMER_ID);
                    }
                    if !self.transition.is_active() {
                        self.full_window_sync();
                    }
                }
                HOVER_WATCH_TIMER_ID => self.on_hover_watch_tick(),
                _ => {}
            },

            WM_HOTKEY => {
                let mk = self.target_monitor_for_input();
                match msg.wParam.0 as i32 {
                    hotkeys::HOTKEY_TOGGLE_SIDEBAR => self.toggle_sidebar_for_monitor(mk),
                    hotkeys::HOTKEY_NEXT_STAGE => {
                        let desktop_id = self.active_desktop_id;
                        if let Some(plan) = self
                            .workspaces
                            .get_mut(&(desktop_id, mk))
                            .and_then(|sm| sm.cycle_next())
                        {
                            self.execute_switch_plan(mk, plan);
                        }
                    }
                    hotkeys::HOTKEY_PREV_STAGE => {
                        let desktop_id = self.active_desktop_id;
                        if let Some(plan) = self
                            .workspaces
                            .get_mut(&(desktop_id, mk))
                            .and_then(|sm| sm.cycle_prev())
                        {
                            self.execute_switch_plan(mk, plan);
                        }
                    }
                    _ => {}
                }
            }

            WM_STAGE_TRAY_QUEUED => {
                let event = (msg.lParam.0 & 0xFFFF) as u32;
                match event {
                    WM_RBUTTONUP | WM_CONTEXTMENU => {
                        let cmd = if let Some(ref tray) = self.tray {
                            let mk = self.target_monitor_for_input();
                            let revealed = self.sidebars.get(&mk).is_some_and(|s| s.revealed);
                            tray.show_context_menu(
                                &self.config,
                                revealed,
                                is_run_at_startup_enabled(),
                            )
                        } else {
                            None
                        };
                        if let Some(cmd) = cmd {
                            self.handle_menu_command(cmd);
                        }
                    }
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                        let mk = self.target_monitor_for_input();
                        self.toggle_sidebar_for_monitor(mk);
                    }
                    _ => {}
                }
            }

            WM_COMMAND => {
                let cmd = (msg.wParam.0 & 0xFFFF) as u32;
                self.handle_menu_command(cmd);
            }

            WM_STAGE_TASKBAR_CREATED => {
                self.tray = TrayIcon::new(self.app_hwnd);
            }

            other if self.taskbar_created_msg != 0 && other == self.taskbar_created_msg => {
                self.tray = TrayIcon::new(self.app_hwnd);
            }

            _ => {}
        }
    }

    fn on_hover_watch_tick(&mut self) {
        if !self.config.auto_hide {
            self.stop_hover_watch_if_idle();
            return;
        }

        let mut pt = POINT::default();
        if unsafe { GetCursorPos(&mut pt).is_err() } {
            return;
        }

        let now = Instant::now();
        let mut wake = false;
        let keys: Vec<isize> = self.sidebars.keys().copied().collect();

        for mk in keys {
            let Some(sb) = self.sidebars.get_mut(&mk) else {
                continue;
            };
            if !sb.revealed {
                self.outside_since.remove(&mk);
                continue;
            }

            if sb.is_cursor_inside(pt) {
                self.outside_since.remove(&mk);
                let hover_changed = sb.on_pointer_move(pt.x, pt.y);
                let scrolling = sb.update_edge_scroll(pt);
                if hover_changed || scrolling {
                    wake = true;
                }
            } else {
                let _ = sb.update_edge_scroll(pt);
                if sb.on_pointer_clear_hover() {
                    wake = true;
                }
                match self.outside_since.get(&mk).copied() {
                    None => {
                        self.outside_since.insert(mk, now);
                    }
                    Some(since) if now.duration_since(since).as_millis() >= 260 => {
                        sb.set_revealed(false);
                        self.outside_since.remove(&mk);
                        wake = true;
                    }
                    _ => {}
                }
            }
        }

        if wake {
            self.wake_vsync();
        }
        self.stop_hover_watch_if_idle();
    }

    fn on_vsync_tick(&mut self) {
        let mut cursor = None;
        if self
            .sidebars
            .values()
            .any(|sb| sb.revealed && sb.is_edge_scrolling())
        {
            let mut pt = POINT::default();
            if unsafe { GetCursorPos(&mut pt).is_ok() } {
                cursor = Some(pt);
            }
        }

        let dt = self.clock.tick();
        let mut sidebar_moving = false;
        let mut edge_scrolling = false;

        for sb in self.sidebars.values_mut() {
            if sb.revealed && sb.is_edge_scrolling() {
                if let Some(pt) = cursor {
                    let _ = sb.update_edge_scroll(pt);
                    let _ = sb.on_pointer_move(pt.x, pt.y);
                }
            }
            if sb.step_animations(dt) {
                sidebar_moving = true;
            }
            if sb.is_edge_scrolling() {
                edge_scrolling = true;
            }
        }

        let transition_moving = self.transition.step(dt);

        if !sidebar_moving && !transition_moving && !edge_scrolling {
            self.vsync.set_active(false);
            self.clock.reset();
        }
    }

    fn on_win_event(&mut self, event: u32, hwnd: HWND) {
        if hwnd == self.app_hwnd
            || hwnd == self.transition.hwnd
            || self.sidebars.values().any(|s| s.owns_hwnd(hwnd))
        {
            return;
        }

        if self.transition.is_active() {
            return;
        }

        if event == EVENT_SYSTEM_FOREGROUND {
            let hk = hwnd.0 as isize;
            if let Some(tw) = self.tracked_map.get(&hk).cloned() {
                let desktop = if tw.desktop_id != [0u8; 16] {
                    tw.desktop_id
                } else {
                    self.active_desktop_id
                };
                self.active_desktop_id = desktop;
                if let Some(sm) = self.workspaces.get_mut(&(desktop, tw.monitor_key)) {
                    if sm.on_foreground_changed(hk) {
                        self.sync_sidebar_for_monitor(desktop, tw.monitor_key, true);
                        if self
                            .sidebars
                            .get(&tw.monitor_key)
                            .is_some_and(|s| s.revealed)
                        {
                            self.wake_vsync();
                        }
                        return;
                    }
                }
            }
            // Switched virtual desktop, moved across monitors, or newly opened window: resync cleanly
            self.schedule_debounced_sync();
            return;
        }

        self.schedule_debounced_sync();
    }

    fn schedule_debounced_sync(&self) {
        unsafe {
            SetTimer(self.app_hwnd, DEBOUNCE_SYNC_TIMER_ID, 80, None);
        }
    }

    fn sync_monitors(&mut self) {
        let fresh = enumerate_monitors();
        let keep: HashSet<isize> = fresh.iter().map(|m| m.monitor_key).collect();

        self.sidebars.retain(|k, _| keep.contains(k));
        self.monitors.retain(|k, _| keep.contains(k));
        self.outside_since.retain(|k, _| keep.contains(k));

        if let Some(prim) = fresh
            .iter()
            .find(|m| m.is_primary)
            .or_else(|| fresh.first())
        {
            self.primary_monitor = prim.monitor_key;
        }

        for m in fresh {
            self.monitors.insert(m.monitor_key, m);
            if let Some(sb) = self.sidebars.get_mut(&m.monitor_key) {
                sb.update_monitor(m);
            } else if let Ok(mut sb) = Sidebar::new_for_monitor(self.config.clone(), m) {
                sb.show_initial();
                self.sidebars.insert(m.monitor_key, sb);
            }
        }

        info!(
            "Synchronized displays: {} monitor(s) active",
            self.sidebars.len()
        );
        self.full_window_sync();
        self.wake_vsync();
    }

    fn full_window_sync(&mut self) {
        let ordered = self.tracker.sync_all();
        self.apply_tracked_windows(ordered);
        if self.sidebars.values().any(|s| s.revealed) {
            self.wake_vsync();
        }
    }

    fn apply_tracked_windows(&mut self, ordered: Vec<TrackedWindow>) {
        let fg = unsafe { GetForegroundWindow() };
        let fg_key = (!fg.0.is_null()).then_some(fg.0 as isize);

        // Determine current Virtual Desktop GUID (empty desktop leaves prior desktops' state intact).
        let detected_desktop = fg_key
            .and_then(|k| ordered.iter().find(|w| w.hwnd_key == k))
            .map(|w| w.desktop_id)
            .filter(|id| *id != [0u8; 16])
            .or_else(|| {
                ordered
                    .iter()
                    .map(|w| w.desktop_id)
                    .find(|id| *id != [0u8; 16])
            });

        self.tracked_map = ordered.iter().cloned().map(|w| (w.hwnd_key, w)).collect();

        if ordered.is_empty() {
            // Empty virtual desktop or no open user windows: show clean 0-stage sidebars without erasing saved desktop workspaces.
            self.sync_all_sidebars_from_workspaces(false);
            return;
        }

        let desktop_id = detected_desktop.unwrap_or(self.active_desktop_id);
        self.active_desktop_id = desktop_id;

        let monitor_keys: Vec<isize> = self.sidebars.keys().copied().collect();
        for mk in monitor_keys {
            let mon_wins: Vec<TrackedWindow> = ordered
                .iter()
                .filter(|w| w.monitor_key == mk)
                .cloned()
                .collect();
            let mon_fg = fg_key.filter(|fk| mon_wins.iter().any(|w| w.hwnd_key == *fk));
            let mode = self.config.grouping_mode;
            let sm = self
                .workspaces
                .entry((desktop_id, mk))
                .or_insert_with(|| StageManager::new(mode));
            sm.sync_windows(&mon_wins, mon_fg);
        }

        self.sync_all_sidebars_from_workspaces(true);
    }

    fn sync_all_sidebars_from_workspaces(&mut self, has_current_windows: bool) {
        let desktop_id = self.active_desktop_id;
        let monitor_keys: Vec<isize> = self.sidebars.keys().copied().collect();
        for mk in monitor_keys {
            self.sync_sidebar_for_monitor(desktop_id, mk, has_current_windows);
        }
    }

    fn sync_sidebar_for_monitor(
        &mut self,
        desktop_id: DesktopId,
        mk: isize,
        has_current_windows: bool,
    ) {
        let Some(sb) = self.sidebars.get_mut(&mk) else {
            return;
        };
        if !has_current_windows {
            sb.sync_stages(&[], &self.tracked_map);
            return;
        }
        if let Some(sm) = self.workspaces.get(&(desktop_id, mk)) {
            let stages: Vec<_> = if self.config.include_active_app {
                sm.stages().to_vec()
            } else {
                sm.inactive_stages().into_iter().cloned().collect()
            };
            let refs: Vec<_> = stages.iter().collect();
            sb.sync_stages(&refs, &self.tracked_map);
        } else {
            sb.sync_stages(&[], &self.tracked_map);
        }
    }

    fn execute_switch_plan(&mut self, mk: isize, plan: SwitchPlan) {
        let Some(sb) = self.sidebars.get(&mk) else {
            return;
        };
        let work_area = sb.work_area;
        let from_card = plan
            .from_stage
            .map(|id| sb.card_screen_rect(id))
            .unwrap_or_else(|| default_offscreen_card_rect(&work_area));
        let to_card = sb.card_screen_rect(plan.to_stage);
        let icon_hwnd = sb.icon_hwnd;

        if self.config.transition_overlay_enabled {
            self.transition.start_transition(
                &plan.hide_windows,
                &plan.show_windows,
                from_card,
                to_card,
                &work_area,
                &self.tracked_map,
                icon_hwnd,
            );
        } else {
            for &hk in &plan.hide_windows {
                crate::transition::minimize_instant(HWND(hk as *mut _));
            }
            for &hk in plan.show_windows.iter().rev() {
                let was_max = self.tracked_map.get(&hk).is_some_and(|w| w.was_maximized);
                restore_instant(HWND(hk as *mut _), was_max);
            }
            if let Some(&front) = plan.show_windows.first() {
                unsafe {
                    let _ = SetForegroundWindow(HWND(front as *mut _));
                }
            }
        }

        self.sync_sidebar_for_monitor(self.active_desktop_id, mk, true);
        self.wake_vsync();
    }

    fn merge_stage(&mut self, mk: isize, stage_id: u32) {
        let desktop_id = self.active_desktop_id;
        let added = self
            .workspaces
            .get_mut(&(desktop_id, mk))
            .map(|sm| sm.merge_into_active(stage_id))
            .unwrap_or_default();
        for &hk in added.iter().rev() {
            let was_max = self.tracked_map.get(&hk).is_some_and(|w| w.was_maximized);
            restore_instant(HWND(hk as *mut _), was_max);
        }
        if let Some(&front) = added.first() {
            unsafe {
                let _ = SetForegroundWindow(HWND(front as *mut _));
            }
        }
        self.sync_sidebar_for_monitor(desktop_id, mk, true);
        self.wake_vsync();
    }

    fn toggle_sidebar_for_monitor(&mut self, mk: isize) {
        let next = !self.sidebars.get(&mk).is_some_and(|s| s.revealed);
        if next {
            self.full_window_sync();
            if let Some(sb) = self.sidebars.get_mut(&mk) {
                sb.set_revealed(true);
            }
            self.outside_since.remove(&mk);
            self.ensure_hover_watch();
        } else {
            if let Some(sb) = self.sidebars.get_mut(&mk) {
                sb.set_revealed(false);
            }
            self.outside_since.remove(&mk);
            self.stop_hover_watch_if_idle();
        }
        self.wake_vsync();
    }

    fn restore_all_windows(&mut self) {
        for tw in self.tracked_map.values() {
            restore_instant(tw.hwnd, tw.was_maximized);
        }
        self.full_window_sync();
    }

    fn handle_menu_command(&mut self, cmd: u32) {
        match cmd {
            MENU_TOGGLE_SIDEBAR => {
                let mk = self.target_monitor_for_input();
                self.toggle_sidebar_for_monitor(mk);
            }
            MENU_AUTO_HIDE => {
                self.config.auto_hide = !self.config.auto_hide;
                for sb in self.sidebars.values_mut() {
                    sb.config.auto_hide = self.config.auto_hide;
                    sb.update_geometry_for_config();
                    sb.set_revealed(!self.config.auto_hide);
                }
                self.stop_hover_watch_if_idle();
                self.wake_vsync();
            }
            MENU_TRIGGER_TOP_CORNER => self.set_trigger_mode(TriggerMode::TopCorner),
            MENU_TRIGGER_BOTTOM_CORNER => self.set_trigger_mode(TriggerMode::BottomCorner),
            MENU_TRIGGER_FULL_EDGE => self.set_trigger_mode(TriggerMode::FullEdge),
            MENU_BLUR_BACKDROP => {
                self.config.blur_backdrop = !self.config.blur_backdrop;
                for sb in self.sidebars.values_mut() {
                    sb.set_blur_backdrop(self.config.blur_backdrop);
                }
            }
            MENU_THEME_DARK => self.set_glass_theme(GlassTheme::DarkFrosted),
            MENU_THEME_LIGHT => self.set_glass_theme(GlassTheme::LightFrosted),
            MENU_EDGE_LEFT => self.set_sidebar_edge(SidebarEdge::Left),
            MENU_EDGE_RIGHT => self.set_sidebar_edge(SidebarEdge::Right),
            MENU_GROUP_BY_APP => self.set_grouping_mode(GroupingMode::ByApplication),
            MENU_GROUP_SINGLE => self.set_grouping_mode(GroupingMode::SingleWindow),
            MENU_INCLUDE_ACTIVE => {
                self.config.include_active_app = !self.config.include_active_app;
                for sb in self.sidebars.values_mut() {
                    sb.config.include_active_app = self.config.include_active_app;
                }
                self.sync_all_sidebars_from_workspaces(!self.tracked_map.is_empty());
                self.wake_vsync();
            }
            MENU_RUN_AT_STARTUP => {
                let next = !is_run_at_startup_enabled();
                let _ = set_run_at_startup(next);
            }
            MENU_RESTORE_ALL => self.restore_all_windows(),
            MENU_EXIT => unsafe {
                PostQuitMessage(0);
            },
            _ => {}
        }
    }

    fn set_trigger_mode(&mut self, mode: TriggerMode) {
        self.config.trigger_mode = mode;
        for sb in self.sidebars.values_mut() {
            sb.config.trigger_mode = mode;
            sb.update_geometry_for_config();
        }
    }

    fn set_glass_theme(&mut self, theme: GlassTheme) {
        if self.config.glass_theme == theme {
            return;
        }
        self.config.glass_theme = theme;
        for sb in self.sidebars.values_mut() {
            sb.set_glass_theme(theme);
        }
    }

    fn set_sidebar_edge(&mut self, edge: SidebarEdge) {
        if self.config.sidebar_edge == edge {
            return;
        }
        self.config.sidebar_edge = edge;
        for sb in self.sidebars.values_mut() {
            sb.config.sidebar_edge = edge;
            sb.update_geometry_for_config();
        }
        self.sync_all_sidebars_from_workspaces(!self.tracked_map.is_empty());
        self.wake_vsync();
    }

    fn set_grouping_mode(&mut self, mode: GroupingMode) {
        if self.config.grouping_mode == mode {
            return;
        }
        self.config.grouping_mode = mode;
        let ordered = self.tracker.sync_all();
        let fg = unsafe { GetForegroundWindow() };
        let fg_key = (!fg.0.is_null()).then_some(fg.0 as isize);

        for (&(_, mk), sm) in &mut self.workspaces {
            let mon_wins: Vec<TrackedWindow> = ordered
                .iter()
                .filter(|w| w.monitor_key == mk)
                .cloned()
                .collect();
            let mon_fg = fg_key.filter(|fk| mon_wins.iter().any(|w| w.hwnd_key == *fk));
            sm.set_grouping_mode(mode, &mon_wins, mon_fg);
        }

        self.tracked_map = ordered.into_iter().map(|w| (w.hwnd_key, w)).collect();
        self.sync_all_sidebars_from_workspaces(!self.tracked_map.is_empty());
        self.wake_vsync();
    }
}

fn default_offscreen_card_rect(wa: &RECT) -> RECT {
    RECT {
        left: wa.left + 14,
        top: (wa.top + wa.bottom) / 2 - 55,
        right: wa.left + 170,
        bottom: (wa.top + wa.bottom) / 2 + 55,
    }
}

fn create_app_host_window() -> windows::core::Result<HWND> {
    let hmod = unsafe { GetModuleHandleW(None)? };
    let cls = wide("StageManagerAppHost");
    unsafe {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(app_host_wnd_proc),
            hInstance: hmod.into(),
            lpszClassName: PCWSTR(cls.as_ptr()),
            ..Default::default()
        };
        let _ = RegisterClassExW(&wc);
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            PCWSTR(cls.as_ptr()),
            PCWSTR(wide("Stage Manager Host").as_ptr()),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            hmod,
            None,
        )
    }
}

unsafe extern "system" fn app_host_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_msg = TASKBAR_CREATED_MSG.load(Ordering::Acquire);
    match msg {
        WM_TRAY_ICON => {
            let _ = PostMessageW(hwnd, WM_STAGE_TRAY_QUEUED, wparam, lparam);
            LRESULT(0)
        }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
            let _ = PostMessageW(hwnd, WM_STAGE_DISPLAY_CHANGE, wparam, lparam);
            LRESULT(0)
        }
        other if taskbar_msg != 0 && other == taskbar_msg => {
            let _ = PostMessageW(hwnd, WM_STAGE_TASKBAR_CREATED, wparam, lparam);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn unpack_lparam_coords(lparam: isize) -> (i32, i32) {
    let x = (lparam & 0xFFFF) as u16 as i16 as i32;
    let y = ((lparam >> 16) & 0xFFFF) as u16 as i16 as i32;
    (x, y)
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
