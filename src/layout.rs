//! macOS Stage Manager layout — readable stage cards, infinite circular carousel
//! wrapping for any number of open apps, cascaded previews, and icon positions.

use crate::config::{Config, SidebarEdge};
use crate::stage_manager::{Stage, StageId};
use windows::Win32::Foundation::RECT;

#[derive(Debug, Clone)]
pub struct PreviewWindowLayout {
    pub hwnd_key: isize,
    pub rect: RECT,
    pub opacity: u8,
}

#[derive(Debug, Clone)]
pub struct StageTargetLayout {
    pub stage_id: StageId,
    pub slot_index: usize,
    pub windows: Vec<isize>,
    pub icon_hwnds: Vec<isize>,
    pub window_count: usize,
}

#[derive(Debug, Clone)]
pub struct AnimatedStageBadge {
    pub icon_hwnds: Vec<isize>,
    pub front_preview_rect: RECT,
    pub window_count: usize,
    pub hover_t: f64,
    pub expand_t: f64,
    pub appear_t: f64,
    pub expanded_previews: Vec<(isize, RECT)>,
}

#[derive(Debug, Clone)]
pub struct AnimatedStageFrame {
    pub stage_id: StageId,
    pub hit_rect: RECT,
    pub previews: Vec<PreviewWindowLayout>,
    pub badge: AnimatedStageBadge,
}

#[derive(Debug, Clone, Copy)]
pub struct StageStripMetrics {
    pub slot_left: i32,
    pub slot_right: i32,
    pub slot_h: i32,
    pub pitch: f64,
    pub pad_y: i32,
    pub work_h: i32,
    pub start_y: i32,
    pub cycle_h: f64,
    pub overflows: bool,
}

/// Compute strip sizing and infinite carousel parameters.
///
/// Cards retain a readable minimum size (`~96px * dpi_scale`). When more apps are
/// open than fit vertically, `overflows` becomes `true` and the strip wraps
/// indefinitely in both directions.
pub fn compute_strip_metrics(
    stage_count: usize,
    strip_w: i32,
    work_h: i32,
    dpi_scale: f64,
    config: &Config,
) -> StageStripMetrics {
    let edge_pad = (config.edge_margin as f64 * dpi_scale).round() as i32;
    let inner_reserve = (28.0 * dpi_scale).round() as i32;
    let gap = (config.stage_gap as f64 * dpi_scale).round() as i32;
    let pad_y = (22.0 * dpi_scale).round() as i32;

    let (slot_left, slot_right) = match config.sidebar_edge {
        SidebarEdge::Left => (edge_pad, (strip_w - inner_reserve).max(edge_pad + 80)),
        SidebarEdge::Right => (inner_reserve, (strip_w - edge_pad).max(inner_reserve + 80)),
    };
    let slot_w = slot_right - slot_left;

    let max_slot_h = ((slot_w as f64 * 0.65).round() as i32).clamp(98, 128);
    let min_slot_h = (96.0 * dpi_scale).round() as i32;
    let count = stage_count.max(1) as i32;
    let avail = (work_h - 2 * pad_y - (count - 1) * gap).max(min_slot_h);
    let slot_h = (avail / count).clamp(min_slot_h, max_slot_h);

    let pitch = (slot_h + gap) as f64;
    let total_h = count * slot_h + (count - 1) * gap;
    let usable_h = (work_h - 2 * pad_y).max(slot_h);
    let overflows = stage_count > 1 && total_h > usable_h;

    let start_y = if !overflows {
        (work_h - total_h) / 2
    } else {
        pad_y
    };

    // Ensure cycle_h always exceeds visible height + one card so cards wrap purely off-screen
    let raw_cycle = (stage_count.max(1) as f64) * pitch;
    let cycle_h = if overflows {
        raw_cycle.max(work_h as f64 + slot_h as f64 + 32.0)
    } else {
        raw_cycle
    };

    StageStripMetrics {
        slot_left,
        slot_right,
        slot_h,
        pitch,
        pad_y,
        work_h,
        start_y,
        cycle_h,
        overflows,
    }
}

/// Compute screen-relative vertical top coordinate inside `[0..work_h)` for a stage slot,
/// seamlessly wrapping around infinitely at both top and bottom when `metrics.overflows` is true.
pub fn compute_slot_top(
    slot_index: f64,
    scroll_pos: f64,
    metrics: &StageStripMetrics,
) -> f64 {
    if !metrics.overflows || metrics.cycle_h <= 0.0 {
        return metrics.start_y as f64 + slot_index * metrics.pitch;
    }

    let mut wrapped = (slot_index * metrics.pitch - scroll_pos).rem_euclid(metrics.cycle_h);
    // Split at the exact midpoint of the off-screen dead zone [work_h - pad_y .. cycle_h - slot_h - pad_y]
    let dead_zone = (metrics.cycle_h - (metrics.work_h + metrics.slot_h) as f64).max(0.0);
    let wrap_threshold = (metrics.work_h - metrics.pad_y) as f64 + dead_zone * 0.5;
    if wrapped > wrap_threshold {
        wrapped -= metrics.cycle_h;
    }
    metrics.pad_y as f64 + wrapped
}

pub fn build_stage_targets(
    stages: &[&Stage],
    config: &Config,
    distinct_icon_hwnds: &dyn Fn(&Stage) -> Vec<isize>,
) -> Vec<StageTargetLayout> {
    stages
        .iter()
        .enumerate()
        .map(|(idx, stage)| StageTargetLayout {
            stage_id: stage.id,
            slot_index: idx,
            windows: stage
                .windows
                .iter()
                .copied()
                .take(config.max_previews_per_stage)
                .collect(),
            icon_hwnds: distinct_icon_hwnds(stage),
            window_count: stage.windows.len(),
        })
        .collect()
}

/// Lay out cascaded rounded window previews inside an animated stage slot.
/// When `expand_t` advances from `0.0` to `1.0` on a grouped stage (`windows.len() > 1`),
/// previews smoothly fan out horizontally toward desktop center so all grouped apps are visible.
/// Emits windows in **front-to-back** order so Z-order chaining places frontmost on top.
pub fn layout_stage_cascade(
    windows: &[isize],
    slot_rect: &RECT,
    work_area: &RECT,
    dpi_scale: f64,
    hover_t: f64,
    expand_t: f64,
    edge: SidebarEdge,
    source_sizes: &dyn Fn(isize) -> Option<(i32, i32)>,
) -> Vec<PreviewWindowLayout> {
    let count = windows.len();
    if count == 0 {
        return Vec::new();
    }

    let slot_w = (slot_rect.right - slot_rect.left).max(20);
    let slot_h = (slot_rect.bottom - slot_rect.top).max(20);

    let step_x = ((11.0 + 7.0 * hover_t) * dpi_scale).round() as i32;
    let step_y = ((7.0 + 4.0 * hover_t) * dpi_scale).round() as i32;
    let max_depth = (count - 1) as i32;

    let avail_w = (slot_w - max_depth * step_x).max(36);
    let avail_h = (slot_h - max_depth * step_y).max(28);

    // Precompute natural compact & expanded layouts, then blend by `exp`.
    let exp = if count > 1 {
        expand_t.clamp(0.0, 1.05)
    } else {
        0.0
    };

    let mut aspects = Vec::with_capacity(count);
    for &hk in windows {
        let (sw, sh) = source_sizes(hk).unwrap_or((16, 10));
        aspects.push((sw.max(1) as f64 / sh.max(1) as f64).clamp(0.5, 2.4));
    }

    // Expanded dimensions: full readable cards arranged horizontally side-by-side.
    let exp_gap = (12.0 * dpi_scale).round() as i32;
    let exp_margin = (24.0 * dpi_scale).round() as i32;
    let max_span_w = match edge {
        SidebarEdge::Left => (work_area.right - slot_rect.left - exp_margin).max(slot_w),
        SidebarEdge::Right => (slot_rect.right - work_area.left - exp_margin).max(slot_w),
    };
    let nat_exp_sizes: Vec<(i32, i32)> = aspects
        .iter()
        .map(|&aspect| fit_aspect(slot_w, slot_h, aspect))
        .collect();
    let total_nat_w: i32 = nat_exp_sizes.iter().map(|(w, _)| *w).sum();
    let total_gaps = exp_gap * (count as i32 - 1).max(0);
    let avail_exp_cards_w = (max_span_w - total_gaps).max(count as i32 * 36);
    let exp_scale = if total_nat_w > avail_exp_cards_w && total_nat_w > 0 {
        avail_exp_cards_w as f64 / total_nat_w as f64
    } else {
        1.0
    };

    let slot_cy = (slot_rect.top + slot_rect.bottom) as f64 * 0.5;
    let pad_y = (10.0 * dpi_scale).round() as i32;
    let mut cursor_x = match edge {
        SidebarEdge::Left => slot_rect.left,
        SidebarEdge::Right => slot_rect.right,
    };

    let mut out = Vec::with_capacity(count);
    for (idx, &hk) in windows.iter().enumerate() {
        let depth = idx as i32;
        let scale_depth = 1.0 - (depth as f64) * 0.04;
        let aspect = aspects[idx];

        let (base_w, base_h) = fit_aspect(avail_w, avail_h, aspect);
        let compact_w = ((base_w as f64 * scale_depth).round() as i32).max(24);
        let compact_h = ((base_h as f64 * scale_depth).round() as i32).max(18);
        let compact_x = match edge {
            SidebarEdge::Left => slot_rect.left + depth * step_x,
            SidebarEdge::Right => slot_rect.right - compact_w - depth * step_x,
        };
        let compact_y = slot_rect.top + (max_depth - depth) * step_y + (avail_h - compact_h) / 2;

        let (nat_w, nat_h) = nat_exp_sizes[idx];
        let exp_w = ((nat_w as f64 * exp_scale).round() as i32).max(32);
        let exp_h = ((nat_h as f64 * exp_scale).round() as i32).max(24);
        let exp_x = match edge {
            SidebarEdge::Left => {
                let x = cursor_x;
                cursor_x += exp_w + exp_gap;
                x
            }
            SidebarEdge::Right => {
                let x = cursor_x - exp_w;
                cursor_x = x - exp_gap;
                x
            }
        };
        let min_top = work_area.top + pad_y;
        let max_top = (work_area.bottom - exp_h - pad_y).max(min_top);
        let exp_y = (slot_cy - exp_h as f64 * 0.5)
            .round()
            .clamp(min_top as f64, max_top as f64) as i32;

        let x = lerp_i32(compact_x, exp_x, exp);
        let y = lerp_i32(compact_y, exp_y, exp);
        let pw = lerp_i32(compact_w, exp_w, exp).max(24);
        let ph = lerp_i32(compact_h, exp_h, exp).max(18);

        out.push(PreviewWindowLayout {
            hwnd_key: hk,
            rect: RECT {
                left: x,
                top: y,
                right: x + pw,
                bottom: y + ph,
            },
            opacity: 255,
        });
    }

    out
}

#[inline]
fn lerp_i32(a: i32, b: i32, t: f64) -> i32 {
    (a as f64 + (b - a) as f64 * t).round() as i32
}

pub fn fit_aspect(max_w: i32, max_h: i32, aspect: f64) -> (i32, i32) {
    let w_from_h = (max_h as f64 * aspect).round() as i32;
    if w_from_h <= max_w {
        (w_from_h.max(1), max_h.max(1))
    } else {
        (max_w.max(1), ((max_w as f64 / aspect).round() as i32).max(1))
    }
}

pub fn hit_test_stage_or_window(
    frames: &[AnimatedStageFrame],
    x: i32,
    y: i32,
) -> Option<(StageId, Option<isize>)> {
    for frame in frames {
        for p in &frame.previews {
            if x >= p.rect.left && x < p.rect.right && y >= p.rect.top && y < p.rect.bottom {
                return Some((frame.stage_id, Some(p.hwnd_key)));
            }
        }
        let r = &frame.hit_rect;
        if x >= r.left && x < r.right && y >= r.top && y < r.bottom {
            return Some((frame.stage_id, None));
        }
    }
    None
}

pub fn hit_test_stages(frames: &[AnimatedStageFrame], x: i32, y: i32) -> Option<StageId> {
    hit_test_stage_or_window(frames, x, y).map(|(stage_id, _)| stage_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_aspect_respects_bounds() {
        let (w, h) = fit_aspect(180, 100, 16.0 / 9.0);
        assert!(w <= 180 && h <= 100);
        assert!((w as f64 / h as f64 - 16.0 / 9.0).abs() < 0.05);
    }

    #[test]
    fn centers_stages_when_not_overflowing() {
        let cfg = Config::default();
        let metrics = compute_strip_metrics(2, 210, 900, 1.0, &cfg);
        assert!(!metrics.overflows);
        let y0 = compute_slot_top(0.0, 0.0, &metrics);
        let y1 = compute_slot_top(1.0, 0.0, &metrics);
        let mid = (y0 + y1 + metrics.slot_h as f64) * 0.5;
        assert!((mid - 450.0).abs() <= 2.0);
    }

    #[test]
    fn wraps_indefinitely_both_directions() {
        let cfg = Config::default();
        let metrics = compute_strip_metrics(14, 210, 800, 1.0, &cfg);
        assert!(metrics.overflows);

        // Scroll far forward (bottom-end infinite scroll)
        let ys_fwd: Vec<f64> = (0..14)
            .map(|i| compute_slot_top(i as f64, 5200.0, &metrics))
            .collect();
        assert!(ys_fwd.iter().any(|&y| y >= 0.0 && y < 400.0));
        assert!(ys_fwd.iter().any(|&y| y >= 400.0 && y < 800.0));

        // Scroll far backward (top-end infinite scroll)
        let ys_back: Vec<f64> = (0..14)
            .map(|i| compute_slot_top(i as f64, -5200.0, &metrics))
            .collect();
        assert!(ys_back.iter().any(|&y| y >= 0.0 && y < 400.0));
        assert!(ys_back.iter().any(|&y| y >= 400.0 && y < 800.0));
    }

    #[test]
    fn barely_overflowing_wrap_seam_is_offscreen() {
        let cfg = Config::default();
        let metrics = compute_strip_metrics(9, 210, 900, 1.0, &cfg);
        assert!(metrics.overflows);
        let mut prev_y = compute_slot_top(0.0, 0.0, &metrics);
        for step in 1..1500 {
            let y = compute_slot_top(0.0, step as f64, &metrics);
            let jump = (y - prev_y).abs();
            if jump > 10.0 {
                // Any wrap jump must happen completely outside visible [0..900] bounds
                assert!(prev_y + (metrics.slot_h as f64) <= 0.0 || prev_y >= 900.0);
                assert!(y + (metrics.slot_h as f64) <= 0.0 || y >= 900.0);
            }
            prev_y = y;
        }
    }

    #[test]
    fn cascade_orders_front_to_back() {
        let r = RECT {
            left: 14,
            top: 100,
            right: 180,
            bottom: 210,
        };
        let work = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let list = layout_stage_cascade(&[1, 2, 3], &r, &work, 1.0, 0.0, 0.0, SidebarEdge::Left, &|_| {
            Some((1600, 1000))
        });
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].hwnd_key, 1);
        assert!(list[0].rect.left < list[1].rect.left);
        assert!(list[1].rect.left < list[2].rect.left);
        // Collapsed group cards overlap horizontally
        assert!(list[1].rect.left < list[0].rect.right);
    }

    #[test]
    fn expanded_group_fans_out_without_overlap_and_hits_exact_window() {
        let r = RECT {
            left: 14,
            top: 300,
            right: 180,
            bottom: 410,
        };
        let work = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let list = layout_stage_cascade(
            &[101, 102, 103],
            &r,
            &work,
            1.0,
            1.0,
            1.0,
            SidebarEdge::Left,
            &|_| Some((1600, 1000)),
        );
        assert_eq!(list.len(), 3);
        assert!(list[0].rect.right < list[1].rect.left);
        assert!(list[1].rect.right < list[2].rect.left);

        let frame = AnimatedStageFrame {
            stage_id: 7,
            hit_rect: RECT {
                left: list[0].rect.left - 8,
                top: list[0].rect.top - 8,
                right: list[2].rect.right + 8,
                bottom: list[0].rect.bottom + 8,
            },
            previews: list.clone(),
            badge: AnimatedStageBadge {
                icon_hwnds: vec![101, 102, 103],
                front_preview_rect: list[0].rect,
                window_count: 3,
                hover_t: 1.0,
                expand_t: 1.0,
                appear_t: 1.0,
                expanded_previews: list.iter().map(|p| (p.hwnd_key, p.rect)).collect(),
            },
        };
        let mid_x = (list[1].rect.left + list[1].rect.right) / 2;
        let mid_y = (list[1].rect.top + list[1].rect.bottom) / 2;
        assert_eq!(
            hit_test_stage_or_window(&[frame], mid_x, mid_y),
            Some((7, Some(102)))
        );
    }
}
