# Complete Installation & Setup Guide

This guide walks you through every way to install and run **Stage Manager for Windows**, from one-click installers to building and running from source via `git clone`.

---

## Table of Contents
- [System Requirements](#system-requirements)
- [Method 1: Clone from GitHub & Build (Recommended for Developers)](#method-1-clone-from-github--build-recommended-for-developers)
  - [Step 1: Install Git & Rust (One-Liner via WinGet)](#step-1-install-git--rust-one-liner-via-winget)
  - [Step 2: Clone the Repository](#step-2-clone-the-repository)
  - [Step 3: Run Immediately](#step-3-run-immediately)
  - [Step 4: Build & Install Permanently to Windows](#step-4-build--install-permanently-to-windows)
- [Method 2: Standalone Prebuilt Installer (No Dev Tools Required)](#method-2-standalone-prebuilt-installer-no-dev-tools-required)
- [Method 3: Portable Mode (Zero-Install)](#method-3-portable-mode-zero-install)
- [How to Use & Verify It's Running](#how-to-use--verify-its-running)
- [Controls & Shortcuts Cheat Sheet](#controls--shortcuts-cheat-sheet)
- [Configuring Startup on Windows Boot](#configuring-startup-on-windows-boot)
- [Uninstalling](#uninstalling)
- [Troubleshooting Common Issues](#troubleshooting-common-issues)

---

## System Requirements

| Requirement | Specification |
|---|---|
| **Operating System** | Windows 10 version 20H1 (build 19041) or higher, or Windows 11 |
| **Architecture** | x64 (64-bit AMD/Intel processor) |
| **Desktop Composition** | Desktop Window Manager (DWM) enabled *(default on all modern Windows installations)* |
| **Administrator Rights** | ❌ **Not required** (Everything installs and runs cleanly in user space) |
| **Disk Space** | ~5 MB for installed app (~500 MB temporary during Rust compilation) |

---

## Method 1: Clone from GitHub & Build (Recommended for Developers)

Follow these steps to clone the repository and build Stage Manager from scratch on a clean machine.

### Step 1: Install Git & Rust (One-Liner via WinGet)

If you don't already have Git and Rust installed, open **PowerShell** and run:

```powershell
winget install --id Git.Git -e --source winget; winget install --id Rustlang.Rustup -e --source winget
```

> [!TIP]
> After `winget` completes, **restart your PowerShell window** so the `git` and `cargo` commands are recognized in your PATH. Verify with:
> ```powershell
> git --version
> cargo --version
> ```

---

### Step 2: Clone the Repository

Clone the repository and enter the project folder:

```powershell
git clone https://github.com/fruitsaladwindowsai-zammy/Stage-manager-Rust.git
cd Stage-manager-Rust
```

---

### Step 3: Run Immediately

To compile and launch Stage Manager directly in one command:

```powershell
cargo run --release
```

- Cargo will download the dependencies (`windows` crate 0.58) and compile with full optimizations.
- The Stage Manager squircle icon will appear in your **System Tray** (bottom-right corner next to the clock).
- Move your mouse to the **left edge of your screen** to reveal the Stage Manager strip!

---

### Step 4: Build & Install Permanently to Windows

If you want to create the official Windows setup file that installs to `%LOCALAPPDATA%\StageManager`, registers in your Start Menu, and adds the Windows uninstall entry:

Run these 4 lines in PowerShell:

```powershell
# 1. Close any running instance
Stop-Process -Name stage_manager -Force -ErrorAction SilentlyContinue

# 2. Build the main tray application
cargo build --release --bin stage_manager

# 3. Build the self-contained installer (embeds stage_manager.exe)
$env:STAGE_MANAGER_EMBED_PATH = "target\release\stage_manager.exe"
cargo build --release --bin StageManagerSetup

# 4. Run the installer
.\target\release\StageManagerSetup.exe
```

That's it! Stage Manager is now permanently installed on your system.

---

## Method 2: Standalone Prebuilt Installer (No Dev Tools Required)

If you don't want to install Rust or Git:

1. Go to the [**GitHub Releases**](https://github.com/fruitsaladwindowsai-zammy/Stage-manager-Rust/releases/latest) page.
2. Download **`StageManagerSetup.exe`**.
3. Double-click `StageManagerSetup.exe` to run it:
   - Installs to `%LOCALAPPDATA%\StageManager\stage_manager.exe`
   - Creates a **Start Menu** shortcut (`Stage Manager`)
   - Registers in **Windows Settings → Apps → Installed Apps**
   - Automatically enables **Launch at Windows Startup**
   - Launches Stage Manager straight into the notification tray

#### Silent / Unattended Installation (for scripts):
```powershell
.\StageManagerSetup.exe --silent
```

---

## Method 3: Portable Mode (Zero-Install)

If you want to run Stage Manager from a USB drive or without modifying the Windows registry:

1. Download **`stage_manager.exe`** from [Releases](https://github.com/fruitsaladwindowsai-zammy/Stage-manager-Rust/releases/latest).
2. Place it in any folder (e.g. `C:\Tools\stage_manager.exe`).
3. Double-click to run. It runs completely standalone in your notification tray.

---

## How to Use & Verify It's Running

1. **Check the System Tray:**
   Look at the bottom-right of your taskbar next to the clock. You will see the Stage Manager squircle icon. (If hidden, click the small `^` arrow on the taskbar).

2. **Trigger the Strip:**
   - Move your mouse cursor anywhere along the **left edge of your monitor**.
   - The Stage Manager cards will glide smoothly onto the screen with live GPU window thumbnails.
   - Click any stage card to bring that application to the front!

3. **Hover to Expand Grouped Apps:**
   - If you have multiple windows open for the same app (like 3 Chrome windows or 2 File Explorer folders), hover over the card.
   - The group will fan out smoothly to the right, showing individual squircle icons for every window. Click any one to bring that specific window forward.

---

## Controls & Shortcuts Cheat Sheet

| Action | Control / Shortcut |
|---|---|
| **Reveal Sidebar** | Move mouse to the active screen edge (or corner sensor) |
| **Toggle Sidebar** | `Win + \`` (backtick) or `Alt + \`` |
| **Switch to Stage / Specific Window** | Click on any stage card or expanded window card |
| **Merge Window into Current Stage** | `Shift + Click` on a stage card |
| **Cycle to Next Stage** | `Win + Alt + →` |
| **Cycle to Previous Stage** | `Win + Alt + ←` |
| **Scroll Infinite App Carousel** | Mouse wheel over the strip, or hover the top/bottom screen endpoints |
| **Settings & Configuration** | Right-click the Stage Manager tray icon |

---

## Configuring Startup on Windows Boot

Stage Manager manages startup through the standard Windows user registry key:
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\StageManager`

### Option A: From the System Tray (Easiest)
- Right-click the **Stage Manager** tray icon.
- Click **Run at Windows Startup** to toggle it on or off (a checkmark indicates it will start on boot).

### Option B: Via PowerShell
```powershell
# Check current startup status:
Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "StageManager" -ErrorAction SilentlyContinue

# Disable startup:
Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "StageManager" -ErrorAction SilentlyContinue
```

---

## Uninstalling

### Method A: Through Windows Settings (Recommended)
1. Open **Windows Settings** (`Win + I`).
2. Navigate to **Apps → Installed Apps** (or *Apps & Features* on Windows 10).
3. Search for **Stage Manager**.
4. Click the **⋯** menu button and select **Uninstall**.

### Method B: Via Command Line
```powershell
# From the installer:
StageManagerSetup.exe --uninstall

# Or manually:
Stop-Process -Name stage_manager -Force -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\StageManager"
Remove-Item -Force "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Stage Manager.lnk" -ErrorAction SilentlyContinue
Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "StageManager" -ErrorAction SilentlyContinue
```

---

## Troubleshooting Common Issues

### 1. `Access is denied (os error 5)` during build
- **Cause:** `stage_manager.exe` is already running in the background and locking the binary file.
- **Fix:** Run `Stop-Process -Name stage_manager -Force` before running `cargo build --release`.

### 2. Execution Policy prevents running scripts (`.ps1 cannot be loaded`)
- **Cause:** Windows PowerShell script execution policy is set to `Restricted`.
- **Fix:** Run commands directly in PowerShell as demonstrated in [Step 4](#step-4-build--install-permanently-to-windows), or bypass the policy using:
  ```powershell
  powershell -ExecutionPolicy Bypass -File .\scripts\build-installer.ps1
  ```

### 3. Tray Icon Missing After Windows Explorer Restarts
- Stage Manager listens for the Win32 `TaskbarCreated` message. If Windows Explorer crashes or restarts, Stage Manager automatically re-registers its tray icon within 1 second.

For more technical troubleshooting, see the [docs/TROUBLESHOOTING.md](TROUBLESHOOTING.md) guide.
