# Release Notes

## [0.2.0] — Multi-Monitor Span Isolation & Clean Visuals (2026-11-20)

- **Multi-Monitor Wallpaper Span Isolation**: Cards, Acrylic backdrop blur, and icon layers are clamped strictly within the active monitor's bounds, eliminating visual bleed or splitting onto neighboring monitors when wallpaper is in span mode.
- **Pure Borderless Stage Icons**: Removed background squircle tiles, drop shadows, and outer borders from stage icons, rendering clean application icons with natural alpha.
- **Automated Prerequisite Verification**: Standalone installer checks for 64-bit architecture, Windows 10/11 version, and DWM composition prior to installation.
- **Continuous Automated Release**: Pipeline automatically triggers tests, builds, and GitHub releases on master pushes.

---

## [0.1.0] — Initial Public Release (2024-11-12)

Stage Manager for Windows brings the intuitive, visual stage-based multitasking experience of macOS to Windows 10 and Windows 11 as a native, ultra-lightweight Rust application.

---

### Highlights

- **Hardware-Accelerated Live Window Thumbnails:** Direct DWM Thumbnail API integration (`DwmRegisterThumbnail`) with rounded corners and zero CPU screenshot polling.
- **VSync-Paced Spring Physics:** Every animation is driven by a custom second-order damped spring engine synchronized with the display's vertical blank via `DwmFlush()`.
- **macOS Ribbon Wave Effect:** Hovered stage pops outward toward the user with an active glow while neighboring stages gently curve inward toward the screen bezel.
- **Grouped Application Fan-Out:** Multi-window groups expand horizontally when hovered, displaying individual Direct2D squircle icons for direct window targeting.
- **Infinite Carousel Loop:** The stage strip wraps seamlessly at screen boundaries.
- **Multi-Monitor Support:** Automatically spawns independent Stage Manager strips across all connected monitors with per-monitor DPI scaling.
- **Virtual Desktop Isolation:** Interoperates with the Windows `IVirtualDesktopManager` COM interface.
- **System Tray Management:** Runs silently in the system tray with right-click configuration and startup toggle.
- **Standalone Setup Executable:** Includes `StageManagerSetup.exe`, a self-contained single-file installer.
