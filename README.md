[![CI](../../actions/workflows/release.yml/badge.svg)](../../actions/workflows/release.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE) ![Platform: Windows](https://img.shields.io/badge/Platform-Windows%2010%2F11-blue) ![Language: Rust](https://img.shields.io/badge/Language-Rust-orange)

# Stage Manager for Windows

A native, lightweight **macOS-style Stage Manager** for Windows 10 & Windows 11 written in Rust using the Windows Desktop Window Manager (DWM) Thumbnail API, Direct2D/DirectWrite, and VSync-paced spring physics.

Runs silently as a **Windows system tray app**, supports **multiple monitors**, isolates **Windows Virtual Desktops**, and can launch automatically on **Windows boot**.

## 📸 Screenshots / Demo

> 📸 *Screenshots and GIF demos coming soon — contributions welcome!*

---

## Features

- **Live GPU Window Previews (`DwmRegisterThumbnail`)** — Real-time rounded-corner thumbnails without polling screenshots.
- **Auto-Hide & Smooth Spring Physics** — Slips off-screen when unused and glides smoothly out when your cursor reaches the left/right screen edge (or corner).
- **macOS Curved Ribbon Wave & Infinite Carousel** — Hovered stage pops toward you while neighbors curve gently inward; rests at the top or bottom edge scroll smoothly and wrap indefinitely in both directions.
- **Grouped App Hover Expansion** — Multi-window app groups fan out cleanly horizontally when hovered so every window in the group is visible and individually clickable.
- **Per-Monitor Independent Views** — Spawns an independent Stage Manager strip and trigger zone on **every connected monitor** (with per-monitor DPI scaling and hotplug support), managing only windows on that display.
- **Virtual Desktop Isolation** — Tracks and isolates workspaces per **Windows Virtual Desktop** (`Win+Ctrl+D`, `Win+Tab`); switching to a new virtual desktop starts a fresh workspace without disturbing prior desktops.
- **System Tray & Windows Boot Startup** — Right-click the tray icon to configure side edge, trigger mode, backdrop, grouping, or toggle **Run at Windows Startup**.
- **Standalone Windows Installer (`StageManagerSetup.exe`)** — One-click install to `%LOCALAPPDATA%\StageManager`, Start Menu shortcut, Windows Installed Apps uninstaller, and automatic boot startup.

---

## 📚 Documentation

| Document | Description |
| --- | --- |
| [Architecture](docs/ARCHITECTURE.md) | Module map, data flows, animation loop |
| [Configuration](docs/CONFIGURATION.md) | All settings, tray menu options, registry |
| [Development Guide](docs/DEVELOPMENT.md) | Build from source, module tour, testing |
| [Installation Guide](docs/INSTALLATION.md) | Installer, portable, uninstall |
| [Troubleshooting](docs/TROUBLESHOOTING.md) | Common issues and fixes |

---

## Option 1: Install via Prebuilt Installer (Recommended)

1. Download **`StageManagerSetup.exe`** (or standalone portable `stage_manager.exe`) from the [**Releases**](../../releases) page.
2. Double-click **`StageManagerSetup.exe`**.
   - Installs to `%LOCALAPPDATA%\StageManager\stage_manager.exe`
   - Adds a Start Menu shortcut (**Stage Manager**)
   - Enables **Run at Windows Startup** automatically
   - Registers an uninstaller in **Windows Settings → Apps → Installed Apps**
   - Launches Stage Manager directly into the Windows system tray

*(Silent CLI switches: `StageManagerSetup.exe --silent` and `StageManagerSetup.exe --uninstall`)*

---

## Option 2: Clone & Build from Source

### Prerequisites
- Windows 10 or Windows 11 (DWM desktop composition enabled)
- [Rust toolchain](https://rustup.rs/) (`stable` — works with either `x86_64-pc-windows-msvc` or `x86_64-pc-windows-gnu`)

### Quick Run (Clone & Run)
```powershell
git clone <your-repo-url>
cd "Stage manager"
cargo run --release
```

### Build Standalone Installer (`StageManagerSetup.exe`)
```powershell
# 1. Build the main tray app
cargo build --release --bin stage_manager
# 2. Bundle it into the installer
$env:STAGE_MANAGER_EMBED_PATH = "target\release\stage_manager.exe"
cargo build --release --bin StageManagerSetup
# 3. Collect outputs
New-Item -ItemType Directory -Force -Path dist
Copy-Item target\release\stage_manager.exe dist\
Copy-Item target\release\StageManagerSetup.exe dist\
```

Outputs both `dist\StageManagerSetup.exe` (self-contained single-file installer) and `dist\stage_manager.exe` (portable tray executable).

---

## Controls & Shortcuts

| Action | Control |
| --- | --- |
| **Reveal Stage Manager** | Move mouse to the active screen edge (or top/bottom corner) |
| **Toggle Sidebar** | `Win + \`` (or `Alt + \`` fallback), or click the tray icon |
| **Switch to Stage / Specific Grouped Window** | Click any stage card (or hover a group to fan out & click any window) |
| **Merge App into Active Stage** | `Shift + Click` on a stage card |
| **Cycle Next / Previous Stage** | `Win + Alt + Right` / `Win + Alt + Left` |
| **Scroll Infinite List** | Mouse wheel or hover top / bottom edge endpoints |
| **Configure Settings & Boot Startup** | Right-click the **Stage Manager** icon in the Windows system tray |

---

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [Code of Conduct](CODE_OF_CONDUCT.md).

---

## Tech Stack

- **Language**: Rust (`edition = "2021"`)
- **APIs**: `windows` crate (`0.58`) — DWM Thumbnails, `DwmFlush()` VSync clock, Direct2D + DirectWrite layered icon overlay, `IVirtualDesktopManager`, `EnumDisplayMonitors`, `Shell_NotifyIconW`, Win32 Registry & `IShellLinkW`

---

## License

[MIT](LICENSE)
