# Getting Started

This guide walks you through downloading, installing, and configuring **Stage Manager for Windows** on your machine.

---

## System Requirements

- **Operating System:** Windows 10 version 20H1 (build 19041) or higher, or Windows 11
- **Architecture:** x64 (64-bit Intel or AMD processor)
- **Desktop Window Manager:** Enabled (default on all modern Windows installations)
- **Administrative Privileges:** Not required (installs and runs in user space)

---

## Installation Methods

### Method 1: Prebuilt Installer (Easiest)

1. Head to the [Releases](../../releases/latest) page and download **`StageManagerSetup.exe`**.
2. Double-click `StageManagerSetup.exe` to run the installer:
   - Copies executable to `%LOCALAPPDATA%\StageManager\stage_manager.exe`
   - Creates a Start Menu entry under `Stage Manager`
   - Adds an uninstaller in **Windows Settings → Apps → Installed Apps**
   - Enables **Run at Windows Startup** automatically
   - Starts Stage Manager directly into the system tray
3. *Unattended silent installation:*
   ```powershell
   .\StageManagerSetup.exe --silent
   ```

---

### Method 2: Clone from GitHub & Build from Source

If you prefer building from source, you can set up and run Stage Manager in minutes.

#### 1. Install Prerequisites (if not already installed)
```powershell
winget install --id Git.Git -e --source winget; winget install --id Rustlang.Rustup -e --source winget
```
*Restart your PowerShell terminal after installation.*

#### 2. Clone the Repository
```powershell
git clone https://github.com/fruitsaladwindowsai-zammy/Stage-manager-Rust.git
cd "Stage-manager-Rust"
```

#### 3. Compile and Run
```powershell
cargo run --release
```

#### 4. Build Permanent Installer (Optional)
```powershell
cargo build --release --bin stage_manager
$env:STAGE_MANAGER_EMBED_PATH = "target\release\stage_manager.exe"
cargo build --release --bin StageManagerSetup
.\target\release\StageManagerSetup.exe
```

---

### Method 3: Portable Mode (Zero-Install)

1. Download **`stage_manager.exe`** from the Releases page.
2. Place it in any directory (e.g. `C:\Tools\stage_manager.exe`).
3. Double-click to run. Stage Manager runs silently in your system tray without modifying the Windows registry.

---

## Verifying Stage Manager is Running

Look at the bottom-right corner of your taskbar (next to the clock). You should see the **Stage Manager squircle icon**. If hidden, click the small `^` arrow on your taskbar.

### First Interaction:
1. Move your mouse cursor to the **left edge of your monitor**.
2. The Stage Manager sidebar will smoothly glide onto the screen, showing live thumbnails of your open applications.
3. Click any stage to switch to that application.
4. If an app has multiple windows open, hover over its card to see the windows fan out horizontally!
