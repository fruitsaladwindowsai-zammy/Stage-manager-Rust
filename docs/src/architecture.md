# Architecture & Internals

Stage Manager for Windows is built with clean, safe Rust on top of the native Windows 10 & 11 Win32 and Desktop Window Manager (DWM) APIs.

---

## 1. High-Level Subsystems

```
src/
├── main.rs            # Entry point: DPI awareness, COM initialization, single-instance mutex
├── app.rs             # Multi-monitor coordinator, WinEvent dispatch, workspace routing
├── window_tracker.rs  # Window enumeration, filtering, WinEvent hook lifecycle
├── desktop.rs         # Monitor enumeration, IVirtualDesktopManager COM interop, startup registry
├── stage_manager.rs   # Stage data structures, grouping logic, stage-switching state
├── sidebar.rs         # Per-monitor floating strip window, hover sensors, layout coordination
├── layout.rs          # Pure geometric layout calculations, fan-out positions, hit testing
├── renderer.rs        # Direct2D vector rendering, squircle badge drawing, UpdateLayeredWindow
├── thumbnail.rs       # DwmRegisterThumbnail and DwmUpdateThumbnailProperties wrapper
├── animation.rs       # Damped spring physics engine, FrameClock, VSync synchronization
├── blur.rs            # Windows 11 Acrylic & Mica DWM window attribute helpers
├── hotkeys.rs         # RegisterHotKey bindings with fallback handling
├── tray.rs            # Shell_NotifyIconW, right-click context menu, custom HICON creation
└── bin/setup.rs       # Standalone Windows installer and uninstaller
```

---

## 2. Event-Driven Lifecycle

Stage Manager avoids CPU-heavy polling loops by responding strictly to Windows OS events:

1. **WinEvent Hooks (`SetWinEventHook`):** `EVENT_SYSTEM_FOREGROUND`, `EVENT_OBJECT_LOCATIONCHANGE`, `EVENT_SYSTEM_MINIMIZESTART`/`END`.
2. **Low-Level Pointer Sensor (`WH_MOUSE_LL`):** Monitors cursor coordinates along the active monitor edge to trigger the auto-hide reveal spring.
3. **Display Changes (`WM_DISPLAYCHANGE`):** Re-enumerates monitors upon display hotplug.
4. **Shell Notifications (`WM_TASKBARCREATED`):** Re-registers the system tray icon automatically if Windows Explorer crashes or restarts.

---

## 3. Direct2D Layered Window Compositing

Window badges, squircle process icons, and active hover glow highlights are drawn onto a transparent layered window (`WS_EX_LAYERED | WS_EX_TRANSPARENT`) positioned directly over the thumbnail strip, blitted via `UpdateLayeredWindow` with 32-bit ARGB alpha blending.
