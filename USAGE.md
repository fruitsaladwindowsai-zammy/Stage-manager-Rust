# Usage & Installation Guide

This guide covers complete step-by-step setup, everyday usage, and troubleshooting for **Stage Manager for Windows**.

---

## Table of Contents
- [1. Prerequisites (Clean Windows Machine)](#1-prerequisites-clean-windows-machine)
- [2. Installation Methods](#2-installation-methods)
  - [Option A: Clone from GitHub & Build (Recommended for Developers)](#option-a-clone-from-github--build-recommended-for-developers)
  - [Option B: One-Click Installer (`StageManagerSetup.exe`)](#option-b-one-click-installer-stagemanagersetupexe)
  - [Option C: Portable Mode (Zero-Install)](#option-c-portable-mode-zero-install)
- [3. How Stage Manager Works](#3-how-stage-manager-works)
- [4. Controls & Shortcuts](#4-controls--shortcuts)
- [5. Multi-Monitor Setup](#5-multi-monitor-setup)
- [6. Windows Virtual Desktops](#6-windows-virtual-desktops)
- [7. Auto-Start on Windows Boot](#7-auto-start-on-windows-boot)
- [8. Uninstalling](#8-uninstalling)

---

## 1. Prerequisites (Clean Windows Machine)

Windows does not come with Git or the Rust toolchain preinstalled. On a clean machine, open **PowerShell** and install both in one command via Windows Package Manager:

```powershell
winget install --id Git.Git -e --source winget; winget install --id Rustlang.Rustup -e --source winget
```

> [!NOTE]
> Close and reopen your PowerShell window after running `winget` so `git` and `cargo` are available in your path. Verify:
> ```powershell
> git --version
> cargo --version
> ```

---

## 2. Installation Methods

### Option A: Clone from GitHub & Build (Recommended for Developers)

```powershell
# 1. Clone repository
git clone https://github.com/fruitsaladwindowsai-zammy/Stage-manager-Rust.git
cd "Stage-manager-Rust"

# 2. Run immediately in release mode
cargo run --release
```

#### Build Permanent Windows Installer & Start Menu Shortcut:
```powershell
# 1. Close any running instance
Stop-Process -Name stage_manager -Force -ErrorAction SilentlyContinue

# 2. Compile main tray application
cargo build --release --bin stage_manager

# 3. Build self-contained installer (embeds stage_manager.exe)
$env:STAGE_MANAGER_EMBED_PATH = "target\release\stage_manager.exe"
cargo build --release --bin StageManagerSetup

# 4. Run the installer
.\target\release\StageManagerSetup.exe
```

---

### Option B: One-Click Installer (`StageManagerSetup.exe`)

1. Download **`StageManagerSetup.exe`** from the [GitHub Releases](https://github.com/fruitsaladwindowsai-zammy/Stage-manager-Rust/releases/latest) page.
2. Double-click the installer:
   - Installs to `%LOCALAPPDATA%\StageManager\stage_manager.exe`
   - Adds a **Start Menu** entry under `Stage Manager`
   - Registers an uninstaller in **Windows Settings → Apps → Installed Apps**
   - Enables **Run at Windows Startup** automatically
   - Starts Stage Manager directly into the system tray

#### Unattended Silent Installation:
```powershell
.\StageManagerSetup.exe --silent
```

---

### Option C: Portable Mode (Zero-Install)

1. Download **`stage_manager.exe`** from Releases.
2. Place it in any directory (e.g. `C:\Tools\stage_manager.exe`).
3. Double-click to run. Stage Manager runs silently in the system tray without writing to system directories.

---

## 3. How Stage Manager Works

### The Stage Concept
- A **Stage** represents a focused workspace containing one or more related application windows.
- When you switch stages, inactive windows are cleanly hidden, and the new stage's windows are brought forward together.
- Inactive stages float in an auto-hiding ribbon on the left (or right) side of your screen.

### Live DWM Thumbnails
Unlike older tools that repeatedly take screenshots, Stage Manager registers direct hardware links with the Windows Desktop Window Manager (`DwmRegisterThumbnail`). This ensures:
- **Zero CPU load** during video playback or typing in background windows
- Instant responsiveness with no screenshot lag
- Hardware rounded corners and crisp anti-aliased scaling

### Smooth Spring Physics
Every transition uses second-order damped springs (`f64`) paced by `DwmFlush()` VSync ticks:
- **Hover Pop:** When you point at a stage, it moves forward with a gentle glow.
- **Neighbor Ribbon Wave:** Neighboring stages above and below bend inward toward the screen edge.
- **Group Fan-Out:** Hovering over a multi-window stage fans out individual window preview cards horizontally.

---

## 4. Controls & Shortcuts

| Action | Control / Shortcut |
|---|---|
| **Reveal Sidebar** | Move mouse to screen edge (or corner sensor) |
| **Toggle Visibility** | `Win + \`` (backtick) or `Alt + \`` |
| **Switch Stage** | Click any stage card |
| **Pick Specific Window in Group** | Hover over stage to fan out, then click target window |
| **Merge Window into Current Stage** | `Shift + Click` on any stage card |
| **Cycle to Next Stage** | `Win + Alt + →` |
| **Cycle to Previous Stage** | `Win + Alt + ←` |
| **Infinite Carousel Scroll** | Scroll mouse wheel over strip or hover top/bottom endpoints |
| **Settings Context Menu** | Right-click Stage Manager tray icon |

---

## 5. Multi-Monitor Setup

Stage Manager automatically detects all connected monitors:
- Each display receives its own **independent Stage Manager strip** and mouse trigger zone.
- Applications on Monitor 1 are managed strictly on Monitor 1; applications on Monitor 2 are managed on Monitor 2.
- **Hotplug Support:** Connecting or disconnecting a monitor triggers a `WM_DISPLAYCHANGE` event; Stage Manager instantly enumerates the new display layout without crashing or losing window associations.

---

## 6. Windows Virtual Desktops

Stage Manager integrates with the Windows 10 & 11 `IVirtualDesktopManager` COM interface:
- Each Virtual Desktop maintains its own isolated set of stages.
- When you press `Win + Ctrl + Left` or `Win + Ctrl + Right`, Stage Manager detects the desktop switch and swaps out the stage ribbon to match the newly active virtual desktop.
- Windows created on Desktop 2 will never leak into Desktop 1's stage strip.

---

## 7. Auto-Start on Windows Boot

### Enable via System Tray:
Right-click the Stage Manager tray icon → click **Run at Windows Startup** (a checkmark indicates enabled).

### Enable / Disable via PowerShell:
```powershell
# Enable startup:
$exe = "$env:LOCALAPPDATA\StageManager\stage_manager.exe"
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "StageManager" -Value $exe

# Disable startup:
Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "StageManager" -ErrorAction SilentlyContinue
```

---

## 8. Uninstalling

### Method A: Windows Settings (Recommended)
1. Open **Settings → Apps → Installed Apps**.
2. Search for **Stage Manager**.
3. Click the `⋯` button → **Uninstall**.

### Method B: Silent Command Line Uninstall
```powershell
StageManagerSetup.exe --uninstall
```
