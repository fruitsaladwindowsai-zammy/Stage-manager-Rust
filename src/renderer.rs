//! Direct2D + `UpdateLayeredWindow` renderer for macOS-style overlapping
//! bottom-left squircle app icon badges and optional blurred backdrop rim.

use crate::config::SidebarEdge;
use crate::layout::AnimatedStageBadge;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Bitmap, ID2D1DCRenderTarget, ID2D1Factory, ID2D1SolidColorBrush,
    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, D2D1_BITMAP_PROPERTIES, D2D1_DRAW_TEXT_OPTIONS_CLIP,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA,
    AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC,
    HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DrawIconEx, UpdateLayeredWindow, DI_NORMAL, HICON, ULW_ALPHA,
};

pub struct IconOverlayRenderer {
    _d2d_factory: ID2D1Factory,
    _dwrite_factory: IDWriteFactory,
    dc_target: ID2D1DCRenderTarget,
    badge_format: IDWriteTextFormat,
    mem_dc: HDC,
    dib_bitmap: HBITMAP,
    old_obj: HGDIOBJ,
    buf_w: i32,
    buf_h: i32,
    icon_cache: HashMap<isize, Option<ID2D1Bitmap>>,
}

impl IconOverlayRenderer {
    pub fn new(dpi_scale: f64) -> windows::core::Result<Self> {
        unsafe {
            let d2d_factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dwrite_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

            let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let dc_target = d2d_factory.CreateDCRenderTarget(&rt_props)?;

            let font_name = wide("Segoe UI Variable Text");
            let locale = wide("en-us");
            let badge_format = dwrite_factory.CreateTextFormat(
                PCWSTR(font_name.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                (10.0 * dpi_scale).max(9.0) as f32,
                PCWSTR(locale.as_ptr()),
            )?;
            let _ = badge_format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
            let _ = badge_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = badge_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);

            let mem_dc = CreateCompatibleDC(None);

            Ok(Self {
                _d2d_factory: d2d_factory,
                _dwrite_factory: dwrite_factory,
                dc_target,
                badge_format,
                mem_dc,
                dib_bitmap: HBITMAP::default(),
                old_obj: HGDIOBJ::default(),
                buf_w: 0,
                buf_h: 0,
                icon_cache: HashMap::new(),
            })
        }
    }

    pub fn cache_icon_if_needed(&mut self, hwnd_key: isize, icon: Option<HICON>) {
        if self.icon_cache.contains_key(&hwnd_key) {
            return;
        }
        let bmp = icon.and_then(|h| self.hicon_to_d2d_bitmap(h));
        self.icon_cache.insert(hwnd_key, bmp);
    }

    pub fn retain_icons(&mut self, keep_keys: &HashSet<isize>) {
        self.icon_cache.retain(|k, _| keep_keys.contains(k));
    }

    /// Render overlapping macOS squircle app icons directly into the layered overlay HWND.
    pub fn render_layered(
        &mut self,
        overlay_hwnd: HWND,
        screen_x: i32,
        screen_y: i32,
        width: i32,
        height: i32,
        dpi_scale: f64,
        edge: SidebarEdge,
        badges: &[AnimatedStageBadge],
    ) {
        if width <= 0 || height <= 0 {
            return;
        }

        if self.buf_w != width || self.buf_h != height {
            self.resize_dib(width, height);
        }

        let rc = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };

        unsafe {
            if self.dc_target.BindDC(self.mem_dc, &rc).is_err() {
                return;
            }

            self.dc_target.BeginDraw();
            self.dc_target.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            let tile_sz = (29.0 * dpi_scale).round() as f32;
            let icon_pad = (2.5 * dpi_scale) as f32;
            let step_x = (22.0 * dpi_scale).round() as f32;
            let radius = (7.5 * dpi_scale) as f32;

            for badge in badges {
                let appear = badge.appear_t.clamp(0.0, 1.0) as f32;
                let h = badge.hover_t.clamp(0.0, 1.0) as f32;
                let expand = badge.expand_t.clamp(0.0, 1.0) as f32;
                let scale = 1.0 + 0.06 * h;
                let sz = tile_sz * scale;
                let r = radius * scale;

                let is_expanded_group = badge.expanded_previews.len() > 1 && expand > 0.02;
                let stack_alpha = if is_expanded_group {
                    appear * (1.0 - expand).clamp(0.0, 1.0)
                } else {
                    appear
                };

                if stack_alpha > 0.02 {
                    let preview = &badge.front_preview_rect;
                    let icons: Vec<isize> = badge.icon_hwnds.iter().copied().take(3).collect();
                    let count = icons.len();
                    if count > 0 {
                        // Anchor bottom-left (or bottom-right) slightly overlapping the preview edge
                        let base_y = preview.bottom as f32 - sz * 0.72;
                        let anchor_x = match edge {
                            SidebarEdge::Left => preview.left as f32 - (5.0 * dpi_scale) as f32,
                            SidebarEdge::Right => {
                                preview.right as f32 - sz + (5.0 * dpi_scale) as f32
                            }
                        };

                        for (i, &hk) in icons.iter().enumerate() {
                            let x = match edge {
                                SidebarEdge::Left => anchor_x + i as f32 * step_x,
                                SidebarEdge::Right => anchor_x - i as f32 * step_x,
                            };
                            self.draw_icon_tile(hk, x, base_y, sz, r, icon_pad, stack_alpha);
                        }

                        // Multi-window count pill when same app has multiple windows
                        if badge.window_count > icons.len() {
                            let pill_sz = (15.0 * dpi_scale) as f32;
                            let px = match edge {
                                SidebarEdge::Left => {
                                    anchor_x + (count.saturating_sub(1) as f32) * step_x + sz
                                        - pill_sz * 0.5
                                }
                                SidebarEdge::Right => {
                                    anchor_x
                                        - (count.saturating_sub(1) as f32) * step_x
                                        - pill_sz * 0.5
                                }
                            };
                            let py = base_y - pill_sz * 0.3;
                            let pill_rr = D2D1_ROUNDED_RECT {
                                rect: D2D_RECT_F {
                                    left: px,
                                    top: py,
                                    right: px + pill_sz,
                                    bottom: py + pill_sz,
                                },
                                radiusX: pill_sz * 0.5,
                                radiusY: pill_sz * 0.5,
                            };
                            if let Ok(bg) = self.brush(0.14, 0.16, 0.22, 0.92 * stack_alpha) {
                                self.dc_target.FillRoundedRectangle(&pill_rr, &bg);
                            }
                            if let Ok(fg) = self.brush(1.0, 1.0, 1.0, 0.98 * stack_alpha) {
                                let txt = wide_no_null(&badge.window_count.to_string());
                                self.dc_target.DrawText(
                                    &txt,
                                    &self.badge_format,
                                    &pill_rr.rect,
                                    &fg,
                                    D2D1_DRAW_TEXT_OPTIONS_CLIP,
                                    DWRITE_MEASURING_MODE_NATURAL,
                                );
                            }
                        }
                    }
                }

                if is_expanded_group {
                    let exp_alpha = appear * expand;
                    for &(hk, ref rect) in &badge.expanded_previews {
                        let base_y = rect.bottom as f32 - sz * 0.72;
                        let x = match edge {
                            SidebarEdge::Left => rect.left as f32 - (5.0 * dpi_scale) as f32,
                            SidebarEdge::Right => rect.right as f32 - sz + (5.0 * dpi_scale) as f32,
                        };
                        self.draw_icon_tile(hk, x, base_y, sz, r, icon_pad, exp_alpha);
                    }
                }
            }

            let _ = self.dc_target.EndDraw(None, None);

            let pt_dst = POINT {
                x: screen_x,
                y: screen_y,
            };
            let pt_src = POINT { x: 0, y: 0 };
            let sz = SIZE {
                cx: width,
                cy: height,
            };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let _ = UpdateLayeredWindow(
                overlay_hwnd,
                None,
                Some(&pt_dst),
                Some(&sz),
                self.mem_dc,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }

    unsafe fn draw_icon_tile(
        &self,
        hk: isize,
        x: f32,
        base_y: f32,
        sz: f32,
        r: f32,
        icon_pad: f32,
        alpha: f32,
    ) {
        let sh_rr = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: x - 1.0,
                top: base_y + 2.0,
                right: x + sz + 1.0,
                bottom: base_y + sz + 3.5,
            },
            radiusX: r + 1.0,
            radiusY: r + 1.0,
        };
        if let Ok(sh) = self.brush(0.0, 0.0, 0.0, 0.34 * alpha) {
            self.dc_target.FillRoundedRectangle(&sh_rr, &sh);
        }

        let tile_rr = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: x,
                top: base_y,
                right: x + sz,
                bottom: base_y + sz,
            },
            radiusX: r,
            radiusY: r,
        };
        if let Ok(tile_bg) = self.brush(0.95, 0.96, 0.98, 0.92 * alpha) {
            self.dc_target.FillRoundedRectangle(&tile_rr, &tile_bg);
        }
        if let Ok(rim) = self.brush(1.0, 1.0, 1.0, 0.65 * alpha) {
            self.dc_target
                .DrawRoundedRectangle(&tile_rr, &rim, 1.0, None);
        }

        if let Some(Some(bmp)) = self.icon_cache.get(&hk) {
            let icon_rect = D2D_RECT_F {
                left: x + icon_pad,
                top: base_y + icon_pad,
                right: x + sz - icon_pad,
                bottom: base_y + sz - icon_pad,
            };
            self.dc_target.DrawBitmap(
                bmp,
                Some(&icon_rect),
                alpha,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
        }
    }

    fn brush(&self, r: f32, g: f32, b: f32, a: f32) -> windows::core::Result<ID2D1SolidColorBrush> {
        unsafe {
            self.dc_target
                .CreateSolidColorBrush(&D2D1_COLOR_F { r, g, b, a }, None)
        }
    }

    fn resize_dib(&mut self, width: i32, height: i32) {
        unsafe {
            if !self.dib_bitmap.is_invalid() {
                if !self.old_obj.is_invalid() {
                    SelectObject(self.mem_dc, self.old_obj);
                }
                let _ = DeleteObject(self.dib_bitmap);
                self.dib_bitmap = HBITMAP::default();
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            if let Ok(bmp) = CreateDIBSection(self.mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
            {
                self.old_obj = SelectObject(self.mem_dc, bmp);
                self.dib_bitmap = bmp;
                self.buf_w = width;
                self.buf_h = height;
            }
        }
    }

    fn hicon_to_d2d_bitmap(&self, hicon: HICON) -> Option<ID2D1Bitmap> {
        const SZ: i32 = 32;
        unsafe {
            let temp_dc = CreateCompatibleDC(None);
            if temp_dc.is_invalid() {
                return None;
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: SZ,
                    biHeight: -SZ,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let dib = match CreateDIBSection(temp_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
                Ok(b) => b,
                Err(_) => {
                    let _ = DeleteDC(temp_dc);
                    return None;
                }
            };
            if bits.is_null() {
                let _ = DeleteObject(dib);
                let _ = DeleteDC(temp_dc);
                return None;
            }

            let old = SelectObject(temp_dc, dib);
            let byte_len = (SZ * SZ * 4) as usize;
            std::ptr::write_bytes(bits as *mut u8, 0, byte_len);

            let _ = DrawIconEx(temp_dc, 0, 0, hicon, SZ, SZ, 0, None, DI_NORMAL);

            let slice = std::slice::from_raw_parts(bits as *const u8, byte_len);
            let mut pixels = slice.to_vec();

            let mut any_alpha = false;
            let mut any_color = false;
            for px in pixels.chunks_exact(4) {
                if px[3] != 0 {
                    any_alpha = true;
                    break;
                }
                if px[0] != 0 || px[1] != 0 || px[2] != 0 {
                    any_color = true;
                }
            }

            for px in pixels.chunks_exact_mut(4) {
                if !any_alpha && any_color {
                    if px[0] != 0 || px[1] != 0 || px[2] != 0 {
                        px[3] = 255;
                    }
                } else {
                    let a = px[3] as u32;
                    px[0] = ((px[0] as u32 * a) / 255) as u8;
                    px[1] = ((px[1] as u32 * a) / 255) as u8;
                    px[2] = ((px[2] as u32 * a) / 255) as u8;
                }
            }

            let props = D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
            };

            let d2d_bmp = self
                .dc_target
                .CreateBitmap(
                    D2D_SIZE_U {
                        width: SZ as u32,
                        height: SZ as u32,
                    },
                    Some(pixels.as_ptr() as *const _),
                    (SZ * 4) as u32,
                    &props,
                )
                .ok();

            SelectObject(temp_dc, old);
            let _ = DeleteObject(dib);
            let _ = DeleteDC(temp_dc);
            d2d_bmp
        }
    }
}

impl Drop for IconOverlayRenderer {
    fn drop(&mut self) {
        unsafe {
            if !self.dib_bitmap.is_invalid() {
                if !self.old_obj.is_invalid() {
                    SelectObject(self.mem_dc, self.old_obj);
                }
                let _ = DeleteObject(self.dib_bitmap);
            }
            if !self.mem_dc.is_invalid() {
                let _ = DeleteDC(self.mem_dc);
            }
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn wide_no_null(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().collect()
}
