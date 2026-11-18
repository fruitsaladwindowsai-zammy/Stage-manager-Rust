# Getting Started

This guide walks you through installing and launching **Stage Manager for Windows**.

---

## System Requirements

- **Operating System:** Windows 10 version 20H1 (build 19041) or higher, or Windows 11
- **Architecture:** x64 (64-bit Intel or AMD processor)
- **Desktop Window Manager:** Enabled (default on all modern Windows installations)
- **Administrative Privileges:** Not required (installs and runs in user space)

---

## Installation Options

### Option 1: Standalone Installer (`StageManagerSetup.exe`)

1. Download **`StageManagerSetup.exe`** from the [GitHub Releases](https://github.com/fruitsaladwindowsai-zammy/Stage-manager-Rust/releases/latest) page.
2. Double-click the installer:
   - Installs to `%LOCALAPPDATA%\StageManager\stage_manager.exe`
   - Creates a **Start Menu** shortcut (`Stage Manager`)
   - Registers in **Windows Settings → Apps → Installed Apps**
   - Automatically enables **Run at Windows Startup**
   - Launches Stage Manager directly into the notification tray
3. *Unattended silent install:*
   ```powershell
   .\StageManagerSetup.exe --silent
   ```

---

### Option 2: Clone from GitHub & Build (Recommended for Developers)

```powershell
# 1. Install Git and Rust (if not already installed)
winget install --id Git.Git -e --source winget; winget install --id Rustlang.Rustup -e --source winget

# 2. Clone repository
git clone https://github.com/fruitsaladwindowsai-zammy/Stage-manager-Rust.git
cd "Stage-manager-Rust"

# 3. Run immediately
cargo run --release
```

---

### Option 3: Portable Mode (Zero-Install)

1. Download **`stage_manager.exe`** from Releases.
2. Place it in any directory (e.g. `C:\Tools\stage_manager.exe`).
3. Double-click to run. Runs entirely in your notification tray without modifying the Windows registry.

---

## Verifying It's Running

Look at the bottom-right corner of your taskbar next to the clock. You should see the **Stage Manager squircle icon**.

- Move your mouse cursor to the **left edge of your monitor** to reveal the Stage Manager strip.
- Click any stage card to bring that application to the front!
