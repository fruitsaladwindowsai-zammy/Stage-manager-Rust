# Release Notes (v0.1.0)

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
