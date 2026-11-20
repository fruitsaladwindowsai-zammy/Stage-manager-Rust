# Release Notes — Stage Manager for Windows

## [0.2.0] — Multi-Monitor Span Isolation & Clean Visuals (2026-11-20)

Stage Manager v0.2.0 introduces multi-monitor boundary isolation for wallpaper span setups, removes background tile boxes for clean borderless application icons, and provides automated prerequisite checking in the installer.

---

### Highlights & Fixes

1. **Multi-Monitor Wallpaper Span Boundary Isolation**:
   - In multi-monitor configurations (especially when Windows wallpaper is set to "Span"), sliding stage ribbons previously moved into coordinates belonging to adjacent monitors.
   - Preview slots, the Acrylic backdrop blur window, and the icon overlay are now clamped strictly to `self.work_area`.
   - `ThumbnailLink::update_clipped` now incorporates horizontal normalized clipping (`clip_left_frac` and `clip_right_frac`) alongside vertical clipping, ensuring cards seamlessly crop against the screen edge when hiding or revealing without leaking onto neighboring displays.

2. **Clean Borderless Application Icons**:
   - Removed the background squircle tile, rim border, and tile shadow box behind icons in the stage view.
   - Applications now display their pure, high-DPI transparent icons directly on the stage ribbon.

3. **Automated Prerequisites Verification in Installer**:
   - `StageManagerSetup.exe` verifies that the OS is Windows 10 build 19041+ or Windows 11, that the system architecture is 64-bit (`x86_64`), and that Desktop Window Manager (DWM) composition is enabled.
   - Supports `--silent` unattended deployment.

4. **Automated Continuous Releases**:
   - Enhanced CI/CD pipeline builds, tests, packages, and releases updated executables and NuGet packages directly on GitHub.

---

## [0.1.0] — Initial Public Release (2024-11-12)

Stage Manager for Windows brings the intuitive, visual stage-based multitasking experience of macOS to Windows 10 and Windows 11 as a native, ultra-lightweight Rust application.

---

### Highlights

- **Hardware-Accelerated Live Window Thumbnails:** Utilizes the Windows DWM Thumbnail API (`DwmRegisterThumbnail`) for real-time window previews with rounded corners, zero screenshot polling, and minimal resource usage.
- **VSync-Paced Spring Physics:** Every animation is driven by a custom second-order damped spring physics engine (`f64`) synchronized with the hardware vertical blank via `DwmFlush()`, delivering fluid 60Hz and 120Hz high-refresh animations.
- **macOS Ribbon Wave Effect:** When hovering over a stage, the hovered card pops outward toward the user with an active glow while neighboring stages gently curve inward toward the screen bezel.
- **Grouped Application Fan-Out:** Hovering over stages containing multiple windows (e.g., several browser windows or document editors) expands the cards horizontally in real time, displaying individual Direct2D squircle icons for direct window selection.
- **Infinite Carousel Loop:** The stage strip wraps seamlessly at screen boundaries. When hovering or scrolling near the top or bottom endpoints, stages loop infinitely with no visual seam.
- **Multi-Monitor Support:** Automatically spawns independent Stage Manager strips across all connected monitors, respecting per-monitor DPI scaling and dynamically adapting to display hotplug events (`WM_DISPLAYCHANGE`).
- **Virtual Desktop Isolation:** Interoperates with the Windows `IVirtualDesktopManager` COM interface, tracking and isolating stages across multiple Windows Virtual Desktops (`Win + Ctrl + Left/Right`).
- **System Tray Management:** Runs silently in the Windows system tray with a right-click context menu for configuring edge docking (Left/Right), trigger modes (Full Edge/Corners), glass themes (Dark Frosted, Light Frosted, Acrylic, Mica), and boot startup.
- **Standalone Setup Executable:** Includes `StageManagerSetup.exe`, a self-contained single-file installer that installs to `%LOCALAPPDATA%\StageManager`, generates Start Menu shortcuts, and registers with Windows Installed Apps.

---

### Fixed & Refined During Initial Stabilization
- Fixed tray context menu dismissal timing by using `TPM_RETURNCMD` and queuing tray messages via `WM_STAGE_TRAY_QUEUED`.
- Removed `WS_EX_NOACTIVATE` from the tray host window to ensure clean focus handoff when clicking menu items.
- Added single-instance mutex (`CreateMutexW`) to prevent duplicate processes from launching simultaneously.
- Added fallback global hotkey binding (`Alt + \``) in case `Win + \`` is reserved by another utility.
- Added custom 32×32 ARGB squircle HICON creation with automatic GDI object cleanup on application exit.

---

### System Requirements
- Windows 10 version 20H1 (build 19041) or higher, or Windows 11
- 64-bit AMD or Intel processor (x64)
- Desktop Window Manager (DWM) composition enabled
- No administrative privileges required for installation or execution
