//! Stage Manager core state machine — stable stage grouping, ordering, merging, and switching.

use crate::config::GroupingMode;
use crate::window_tracker::TrackedWindow;
use std::collections::{HashMap, HashSet};

pub type StageId = u32;

/// A logical workspace stage consisting of one or more application windows.
#[derive(Debug, Clone)]
pub struct Stage {
    pub id: StageId,
    pub group_key: String,
    pub label: String,
    /// Window `hwnd_key`s belonging to this stage (frontmost first).
    pub windows: Vec<isize>,
}

/// Result of initiating a stage switch, specifying which windows transition out and in.
#[derive(Debug, Clone)]
pub struct SwitchPlan {
    pub from_stage: Option<StageId>,
    pub to_stage: StageId,
    pub hide_windows: Vec<isize>,
    pub show_windows: Vec<isize>,
}

/// Manages stages with stable IDs and deterministic ordering.
pub struct StageManager {
    stages: Vec<Stage>,
    active_stage: Option<StageId>,
    next_id: StageId,
    mode: GroupingMode,
    /// Maps `hwnd_key` -> `StageId` for fast lookup and manual-merge persistence.
    window_to_stage: HashMap<isize, StageId>,
}

impl StageManager {
    pub fn new(mode: GroupingMode) -> Self {
        Self {
            stages: Vec::new(),
            active_stage: None,
            next_id: 1,
            mode,
            window_to_stage: HashMap::new(),
        }
    }

    /// Change grouping mode and re-partition existing windows cleanly.
    pub fn set_grouping_mode(&mut self, mode: GroupingMode, tracked: &[TrackedWindow], foreground: Option<isize>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.stages.clear();
        self.window_to_stage.clear();
        self.active_stage = None;
        self.sync_windows(tracked, foreground);
    }

    /// Incrementally synchronize stages with the current set of tracked windows.
    ///
    /// Preserves existing `StageId`s, custom merged stages, and card order so
    /// sidebar cards never flicker or shuffle unexpectedly.
    pub fn sync_windows(&mut self, tracked: &[TrackedWindow], foreground: Option<isize>) {
        let by_key: HashMap<isize, &TrackedWindow> =
            tracked.iter().map(|w| (w.hwnd_key, w)).collect();

        // 1. Remove closed windows from existing stages
        for stage in &mut self.stages {
            stage.windows.retain(|hk| by_key.contains_key(hk));
        }
        self.stages.retain(|s| !s.windows.is_empty());

        // Rebuild window_to_stage index
        self.window_to_stage.clear();
        for stage in &self.stages {
            for &hk in &stage.windows {
                self.window_to_stage.insert(hk, stage.id);
            }
        }

        // Update active_stage if it vanished
        if let Some(active) = self.active_stage {
            if !self.stages.iter().any(|s| s.id == active) {
                self.active_stage = None;
            }
        }

        // 2. Add newly discovered windows
        for tw in tracked {
            if self.window_to_stage.contains_key(&tw.hwnd_key) {
                // Refresh label if SingleWindow mode
                if self.mode == GroupingMode::SingleWindow {
                    if let Some(&sid) = self.window_to_stage.get(&tw.hwnd_key) {
                        if let Some(s) = self.stages.iter_mut().find(|s| s.id == sid) {
                            s.label = truncate_title(&tw.title, 26);
                        }
                    }
                }
                continue;
            }

            let group_key = self.group_key_for(tw);
            let label = self.label_for(tw);

            if self.mode == GroupingMode::ByApplication {
                if let Some(existing) = self.stages.iter_mut().find(|s| s.group_key == group_key) {
                    existing.windows.push(tw.hwnd_key);
                    self.window_to_stage.insert(tw.hwnd_key, existing.id);
                    continue;
                }
            }

            let id = self.next_id;
            self.next_id += 1;
            self.stages.push(Stage {
                id,
                group_key,
                label,
                windows: vec![tw.hwnd_key],
            });
            self.window_to_stage.insert(tw.hwnd_key, id);
        }

        // 3. Synchronize active stage with current foreground window (if any)
        if let Some(fg) = foreground {
            if let Some(&sid) = self.window_to_stage.get(&fg) {
                self.active_stage = Some(sid);
            }
        }

        // Fallback if no active stage yet: pick first non-minimized stage, or first stage
        if self.active_stage.is_none() && !self.stages.is_empty() {
            let non_min = self
                .stages
                .iter()
                .find(|s| s.windows.iter().any(|hk| by_key.get(hk).is_some_and(|w| !w.is_minimized)));
            self.active_stage = Some(non_min.unwrap_or(&self.stages[0]).id);
        }
    }

    /// Update active stage when a foreground change WinEvent arrives for `hwnd_key`.
    /// Returns `true` if the active stage actually changed.
    pub fn on_foreground_changed(&mut self, hwnd_key: isize) -> bool {
        if let Some(&sid) = self.window_to_stage.get(&hwnd_key) {
            if self.active_stage != Some(sid) {
                self.active_stage = Some(sid);
                // Move focused window to front of its stage list
                if let Some(stage) = self.stages.iter_mut().find(|s| s.id == sid) {
                    stage.windows.retain(|&w| w != hwnd_key);
                    stage.windows.insert(0, hwnd_key);
                }
                return true;
            }
        }
        false
    }

    /// Plan a switch from the current active stage to `target_id`.
    pub fn switch_to(&mut self, target_id: StageId) -> Option<SwitchPlan> {
        if Some(target_id) == self.active_stage {
            return None;
        }
        let target_windows = self.stages.iter().find(|s| s.id == target_id)?.windows.clone();
        let from_stage = self.active_stage;

        let target_set: HashSet<isize> = target_windows.iter().copied().collect();
        let hide_windows = if let Some(from_id) = from_stage {
            self.stages
                .iter()
                .find(|s| s.id == from_id)
                .map(|s| {
                    s.windows
                        .iter()
                        .copied()
                        .filter(|w| !target_set.contains(w))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        self.active_stage = Some(target_id);

        Some(SwitchPlan {
            from_stage,
            to_stage: target_id,
            hide_windows,
            show_windows: target_windows,
        })
    }

    /// Switch to `target_id`, promoting a specific clicked grouped `focus_hwnd` to frontmost first.
    pub fn switch_to_window(
        &mut self,
        target_id: StageId,
        focus_hwnd: Option<isize>,
    ) -> Option<SwitchPlan> {
        let mut promoted_active = false;
        if let Some(hk) = focus_hwnd {
            if let Some(stage) = self.stages.iter_mut().find(|s| s.id == target_id) {
                if let Some(pos) = stage.windows.iter().position(|&w| w == hk) {
                    if pos > 0 {
                        stage.windows.remove(pos);
                        stage.windows.insert(0, hk);
                        promoted_active = Some(target_id) == self.active_stage;
                    }
                }
            }
        }
        if promoted_active {
            let show_windows = self.stages.iter().find(|s| s.id == target_id)?.windows.clone();
            return Some(SwitchPlan {
                from_stage: Some(target_id),
                to_stage: target_id,
                hide_windows: Vec::new(),
                show_windows,
            });
        }
        self.switch_to(target_id)
    }

    /// Merge another stage's windows into the currently active stage (Shift+Click behavior).
    /// Returns the list of newly revealed `hwnd_key`s from the merged stage.
    pub fn merge_into_active(&mut self, source_id: StageId) -> Vec<isize> {
        let Some(active_id) = self.active_stage else {
            return Vec::new();
        };
        if active_id == source_id {
            return Vec::new();
        }

        let Some(src_pos) = self.stages.iter().position(|s| s.id == source_id) else {
            return Vec::new();
        };
        let src_stage = self.stages.remove(src_pos);
        let added = src_stage.windows.clone();

        if let Some(active) = self.stages.iter_mut().find(|s| s.id == active_id) {
            for &hk in &added {
                if !active.windows.contains(&hk) {
                    active.windows.push(hk);
                }
                self.window_to_stage.insert(hk, active_id);
            }
            if !active.label.contains('+') {
                active.label = format!("{} + {}", active.label, src_stage.label);
            }
        }

        added
    }

    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }

    pub fn inactive_stages(&self) -> Vec<&Stage> {
        self.stages
            .iter()
            .filter(|s| Some(s.id) != self.active_stage)
            .collect()
    }

    #[cfg(test)]
    pub fn stage_for_window(&self, hwnd_key: isize) -> Option<StageId> {
        self.window_to_stage.get(&hwnd_key).copied()
    }

    pub fn cycle_next(&mut self) -> Option<SwitchPlan> {
        if self.stages.len() < 2 {
            return None;
        }
        let cur = self.active_stage.unwrap_or(self.stages[0].id);
        let idx = self.stages.iter().position(|s| s.id == cur).unwrap_or(0);
        let next_id = self.stages[(idx + 1) % self.stages.len()].id;
        self.switch_to(next_id)
    }

    pub fn cycle_prev(&mut self) -> Option<SwitchPlan> {
        if self.stages.len() < 2 {
            return None;
        }
        let cur = self.active_stage.unwrap_or(self.stages[0].id);
        let idx = self.stages.iter().position(|s| s.id == cur).unwrap_or(0);
        let prev_idx = if idx == 0 { self.stages.len() - 1 } else { idx - 1 };
        let prev_id = self.stages[prev_idx].id;
        self.switch_to(prev_id)
    }

    fn group_key_for(&self, tw: &TrackedWindow) -> String {
        match self.mode {
            GroupingMode::ByApplication => {
                if tw.exe_name.is_empty() {
                    format!("hwnd:{}", tw.hwnd_key)
                } else {
                    format!("app:{}", tw.exe_name.to_lowercase())
                }
            }
            GroupingMode::SingleWindow => format!("hwnd:{}", tw.hwnd_key),
        }
    }

    fn label_for(&self, tw: &TrackedWindow) -> String {
        match self.mode {
            GroupingMode::ByApplication => friendly_app_name(&tw.exe_name, &tw.title),
            GroupingMode::SingleWindow => truncate_title(&tw.title, 26),
        }
    }
}

fn friendly_app_name(exe_name: &str, title_fallback: &str) -> String {
    if exe_name.is_empty() {
        return truncate_title(title_fallback, 22);
    }
    let stem = exe_name
        .strip_suffix(".exe")
        .or_else(|| exe_name.strip_suffix(".EXE"))
        .unwrap_or(exe_name);

    match stem.to_lowercase().as_str() {
        "code" => "VS Code".to_string(),
        "msedge" => "Edge".to_string(),
        "chrome" => "Chrome".to_string(),
        "firefox" => "Firefox".to_string(),
        "windowsterminal" => "Terminal".to_string(),
        "explorer" => "Explorer".to_string(),
        "notepad" => "Notepad".to_string(),
        "discord" => "Discord".to_string(),
        "spotify" => "Spotify".to_string(),
        "slack" => "Slack".to_string(),
        "powershell" | "pwsh" => "PowerShell".to_string(),
        _ => {
            let mut chars = stem.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
    }
}

fn truncate_title(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let prefix: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{prefix}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use windows::Win32::Foundation::{HWND, RECT};

    fn fake_win(id: isize, exe: &str, title: &str) -> TrackedWindow {
        TrackedWindow {
            hwnd: HWND(id as *mut _),
            hwnd_key: id,
            title: title.to_string(),
            exe_name: exe.to_string(),
            exe_path: PathBuf::new(),
            visual_rect: RECT::default(),
            window_rect: RECT::default(),
            frame_insets: Default::default(),
            normal_rect: RECT::default(),
            was_maximized: false,
            is_minimized: false,
            icon: None,
            owns_icon: false,
            monitor_key: 0,
            desktop_id: [0; 16],
        }
    }

    #[test]
    fn preserves_stage_ids_across_syncs() {
        let mut mgr = StageManager::new(GroupingMode::ByApplication);
        let wins = vec![
            fake_win(1, "chrome.exe", "Google"),
            fake_win(2, "Code.exe", "main.rs"),
        ];
        mgr.sync_windows(&wins, Some(1));
        assert_eq!(mgr.stages().len(), 2);
        let chrome_id = mgr.stage_for_window(1).unwrap();
        let code_id = mgr.stage_for_window(2).unwrap();

        // Add another chrome window and sync again
        let wins2 = vec![
            fake_win(1, "chrome.exe", "Google"),
            fake_win(3, "chrome.exe", "Docs"),
            fake_win(2, "Code.exe", "main.rs"),
        ];
        mgr.sync_windows(&wins2, Some(1));
        assert_eq!(mgr.stages().len(), 2);
        assert_eq!(mgr.stage_for_window(1), Some(chrome_id));
        assert_eq!(mgr.stage_for_window(3), Some(chrome_id));
        assert_eq!(mgr.stage_for_window(2), Some(code_id));
    }

    #[test]
    fn switch_and_merge_work() {
        let mut mgr = StageManager::new(GroupingMode::ByApplication);
        let wins = vec![
            fake_win(1, "chrome.exe", "Tab 1"),
            fake_win(2, "Code.exe", "Repo"),
            fake_win(3, "WindowsTerminal.exe", "Shell"),
        ];
        mgr.sync_windows(&wins, Some(1));
        let code_id = mgr.stage_for_window(2).unwrap();
        let term_id = mgr.stage_for_window(3).unwrap();

        let plan = mgr.switch_to(code_id).unwrap();
        assert_eq!(plan.hide_windows, vec![1]);
        assert_eq!(plan.show_windows, vec![2]);

        let merged = mgr.merge_into_active(term_id);
        assert_eq!(merged, vec![3]);
        assert_eq!(mgr.stage_for_window(3), Some(code_id));
        assert_eq!(mgr.stages().len(), 2);
    }

    #[test]
    fn switch_to_window_promotes_clicked_grouped_window() {
        let mut mgr = StageManager::new(GroupingMode::ByApplication);
        let wins = vec![
            fake_win(1, "Code.exe", "Active"),
            fake_win(10, "chrome.exe", "Tab A"),
            fake_win(11, "chrome.exe", "Tab B"),
            fake_win(12, "chrome.exe", "Tab C"),
        ];
        mgr.sync_windows(&wins, Some(1));
        let chrome_id = mgr.stage_for_window(10).unwrap();

        let plan = mgr.switch_to_window(chrome_id, Some(12)).unwrap();
        assert_eq!(plan.show_windows, vec![12, 10, 11]);
        assert_eq!(plan.hide_windows, vec![1]);
    }
}
