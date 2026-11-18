# DWM Thumbnails & Direct2D Rendering

A core distinction of Stage Manager for Windows is its use of hardware-accelerated OS composition rather than screenshot captures.

---

## 1. DWM Hardware Compositing

When an application window is parked in an inactive stage, Stage Manager links it to the sidebar window using `DwmRegisterThumbnail`:

```rust
DwmRegisterThumbnail(sidebar_hwnd, target_hwnd, &mut thumbnail_id);
```

### Advantages of DWM Thumbnails:
- **Zero Memory Copies:** The GPU compositor blits the live window buffer directly to the destination rectangle during hardware composition.
- **Live Updating:** Videos playing in background browser tabs or text being typed into editors animate smoothly inside the thumbnail preview.
- **Hardware Anti-Aliasing & Rounded Corners:** Destination rectangles are clipped with rounded corners using DWM visual styling flags (`DWM_TNP_RECTDESTINATION`).

---

## 2. Direct2D Layered Window Architecture

Window badges, squircle process icons, and active hover glow highlights cannot be drawn directly over DWM thumbnails via standard GDI without tearing. Stage Manager solves this by implementing a **Direct2D Layered Window overlay**:

1. A transparent layered window (`WS_EX_LAYERED | WS_EX_TRANSPARENT`) sits directly over the thumbnail strip.
2. The Direct2D render target draws vector squircles, drop shadows, and 32-bit ARGB anti-aliased process icons.
3. The resulting frame is transferred to the window manager using `UpdateLayeredWindow`:
   ```rust
   UpdateLayeredWindow(
       overlay_hwnd,
       hdc_screen,
       &pt_dst,
       &size_dst,
       hdc_mem,
       &pt_src,
       0,
       &blend_func, // AC_SRC_OVER with AC_SRC_ALPHA
       ULW_ALPHA,
   );
   ```

---

## 3. High-DPI Vector Scaling

When running on displays with different DPI scalings (e.g., a 100% 1080p side monitor and a 175% 4K primary monitor):
- Icons are loaded from executable resources and scaled with Direct2D bicubic filtering.
- All badge borders, shadow radii, and corner curves scale proportionally, ensuring crisp clarity across all screen configurations.
