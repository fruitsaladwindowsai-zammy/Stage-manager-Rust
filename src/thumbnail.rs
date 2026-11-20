//! DWM Live Thumbnail wrapper — registers and updates GPU-composited window previews.

use crate::window_tracker::FrameInsets;
use log::debug;
use std::collections::HashMap;
use windows::Win32::Foundation::{BOOL, HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmQueryThumbnailSourceSize, DwmRegisterThumbnail, DwmUnregisterThumbnail,
    DwmUpdateThumbnailProperties, DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY,
    DWM_TNP_RECTDESTINATION, DWM_TNP_RECTSOURCE, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
};

/// RAII wrapper over a DWM thumbnail handle (`isize`).
pub struct ThumbnailLink {
    handle: isize,
}

impl ThumbnailLink {
    pub fn new(dest_hwnd: HWND, src_hwnd: HWND) -> windows::core::Result<Self> {
        let handle = unsafe { DwmRegisterThumbnail(dest_hwnd, src_hwnd)? };
        debug!(
            "DWM thumbnail registered: src={:?} -> handle={}",
            src_hwnd, handle
        );
        Ok(Self { handle })
    }

    pub fn source_size(&self) -> windows::core::Result<(i32, i32)> {
        let size = unsafe { DwmQueryThumbnailSourceSize(self.handle)? };
        Ok((size.cx, size.cy))
    }

    /// Update destination rect, cropped source rect (from `FrameInsets`), opacity, and visibility.
    pub fn update(
        &self,
        dest_rect: RECT,
        insets: Option<FrameInsets>,
        opacity: u8,
        visible: bool,
    ) -> windows::core::Result<()> {
        self.update_clipped(dest_rect, insets, 0.0, 0.0, 0.0, 0.0, opacity, visible)
    }

    /// Update with optional normalized horizontal & vertical clipping
    /// (`clip_left_frac`, `clip_right_frac`, `clip_top_frac`, `clip_bot_frac` in 0..1)
    /// so thumbnails sliding or scrolling past the monitor work area clip smoothly
    /// without bleeding onto adjacent monitors or squishing.
    #[allow(clippy::too_many_arguments)]
    pub fn update_clipped(
        &self,
        dest_rect: RECT,
        insets: Option<FrameInsets>,
        clip_left_frac: f64,
        clip_right_frac: f64,
        clip_top_frac: f64,
        clip_bot_frac: f64,
        opacity: u8,
        visible: bool,
    ) -> windows::core::Result<()> {
        let mut flags = DWM_TNP_RECTDESTINATION
            | DWM_TNP_VISIBLE
            | DWM_TNP_OPACITY
            | DWM_TNP_SOURCECLIENTAREAONLY;
        let mut rc_source = RECT::default();

        if let Ok((src_w, src_h)) = self.source_size() {
            if src_w > 16 && src_h > 16 {
                let ins = insets.unwrap_or_default();
                let base_l = ins.left.max(0);
                let base_t = ins.top.max(0);
                let base_r = (src_w - ins.right.max(0)).max(base_l + 1);
                let base_b = (src_h - ins.bottom.max(0)).max(base_t + 1);
                let span_w = (base_r - base_l) as f64;
                let span_h = (base_b - base_t) as f64;
                let cl = base_l + (clip_left_frac.clamp(0.0, 0.98) * span_w).round() as i32;
                let cr = base_r - (clip_right_frac.clamp(0.0, 0.98) * span_w).round() as i32;
                let ct = base_t + (clip_top_frac.clamp(0.0, 0.98) * span_h).round() as i32;
                let cb = base_b - (clip_bot_frac.clamp(0.0, 0.98) * span_h).round() as i32;
                if cr > cl && cb > ct {
                    rc_source = RECT {
                        left: cl,
                        top: ct,
                        right: cr,
                        bottom: cb,
                    };
                    flags |= DWM_TNP_RECTSOURCE;
                }
            }
        }

        let props = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: flags,
            rcDestination: dest_rect,
            rcSource: rc_source,
            opacity,
            fVisible: BOOL::from(visible),
            fSourceClientAreaOnly: BOOL::from(false),
        };

        unsafe { DwmUpdateThumbnailProperties(self.handle, &props) }
    }
}

impl Drop for ThumbnailLink {
    fn drop(&mut self) {
        unsafe {
            let _ = DwmUnregisterThumbnail(self.handle);
        }
    }
}

/// Manages active DWM thumbnails for a destination host window (sidebar or transition overlay).
pub struct ThumbnailManager {
    dest_hwnd: HWND,
    links: HashMap<isize, ThumbnailLink>,
}

impl ThumbnailManager {
    pub fn new(dest_hwnd: HWND) -> Self {
        Self {
            dest_hwnd,
            links: HashMap::new(),
        }
    }

    /// Ensure a thumbnail link exists for `src_hwnd`.
    pub fn ensure(&mut self, src_hwnd: HWND) -> bool {
        let key = src_hwnd.0 as isize;
        if self.links.contains_key(&key) {
            return true;
        }
        if let Ok(link) = ThumbnailLink::new(self.dest_hwnd, src_hwnd) {
            self.links.insert(key, link);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.links.clear();
    }

    pub fn source_size(&self, hwnd_key: isize, insets: Option<FrameInsets>) -> Option<(i32, i32)> {
        let link = self.links.get(&hwnd_key)?;
        let (w, h) = link.source_size().ok()?;
        if w <= 0 || h <= 0 {
            return None;
        }
        if let Some(ins) = insets {
            let cw = (w - ins.left - ins.right).max(1);
            let ch = (h - ins.top - ins.bottom).max(1);
            Some((cw, ch))
        } else {
            Some((w, h))
        }
    }

    pub fn update(
        &self,
        hwnd_key: isize,
        dest_rect: RECT,
        insets: Option<FrameInsets>,
        opacity: u8,
        visible: bool,
    ) {
        if let Some(link) = self.links.get(&hwnd_key) {
            let _ = link.update(dest_rect, insets, opacity, visible);
        }
    }
}
