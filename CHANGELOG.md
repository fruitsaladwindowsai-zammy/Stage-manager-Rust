# Changelog

All notable changes to Stage Manager for Windows will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-11-20

### Added
- **Automated Prerequisite Verification in Setup Installer**: `StageManagerSetup.exe` automatically checks for Windows 10/11 version, 64-bit architecture, and Desktop Window Manager (DWM) composition before installation, offering interactive or silent (`--silent`) operation.
- **Automated Multi-Channel Release Pipeline**: Pushes to `master` and version tags automatically compile, test, package NuGet artifacts, and publish GitHub releases with binary assets.

### Fixed
- **Multi-Monitor Wallpaper Span Mode Boundary Bleed**: Clamped preview slots, Acrylic blur backdrops, and Direct2D icon overlay windows strictly within each monitor's `work_area`. Implemented normalized horizontal cropping (`clip_left_frac` and `clip_right_frac`) in `ThumbnailLink::update_clipped` so cards sliding off-screen or returning never spill across displays in wallpaper span mode.
- **Pure Borderless Stage Icons**: Removed background squircle tile fill, outer rim border, and tile drop-shadow box from `draw_icon_tile` in `renderer.rs`. Application icons now render crisply with natural alpha without enclosing boxes or borders.
- **Preview Creation Coordinates**: Created preview slots within the monitor's work area rather than negative coordinates to prevent ghosting on left-aligned secondary monitors.

## [0.1.0] - 2024-11-12

### Added

- Live GPU window previews via DWM Thumbnail API (`DwmRegisterThumbnail`)
- macOS-style spring physics animation with `DwmFlush`-paced VSync
- Auto-hide sidebar with full-edge, top-corner, and bottom-corner trigger modes
- Infinite carousel scroll at top/bottom endpoints (wraps indefinitely)
- Grouped app hover fan-out: multi-window groups expand horizontally with individual squircle icons
- Smooth curved wave: hovered stage pops toward you while neighbors curve inward
- Per-monitor independent sidebars with per-monitor DPI scaling
- Monitor hotplug support (`WM_DISPLAYCHANGE`)
- Windows Virtual Desktop isolation via `IVirtualDesktopManager`
- System tray icon with right-click context menu
- Configurable: sidebar edge (Left/Right), trigger mode, glass theme, grouping mode, backdrop blur
- Windows boot startup toggle (`HKCU` Run registry key)
- Standalone Windows installer (`StageManagerSetup.exe`): installs to `%LOCALAPPDATA%`, Start Menu shortcut, Windows uninstaller registration
- Silent install flag (`--silent`) and uninstall flag (`--uninstall`)
- Single-instance guard (`CreateMutexW`)
- Global hotkeys: `Win+`` ` toggle, `Win+Alt+Left/Right` stage cycle
- Shift+Click to merge windows into the active stage
- Direct2D squircle icon overlay via `UpdateLayeredWindow`
- Windows 11 Acrylic/Mica glass themes via `DwmSetWindowAttribute`

[Unreleased]: ../../compare/v0.1.0...HEAD
[0.1.0]: ../../releases/tag/v0.1.0
