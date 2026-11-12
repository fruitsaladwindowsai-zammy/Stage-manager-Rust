# Changelog

All notable changes to Stage Manager for Windows will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
