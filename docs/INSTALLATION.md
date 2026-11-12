# Installation Guide

> [!NOTE]
> Stage Manager for Windows requires no administrator rights. Everything installs and runs in your user account.

## System Requirements

| Requirement | Minimum |
|---|---|
| **OS** | Windows 10 version 20H1 (build 19041) or Windows 11 |
| **Architecture** | x64 (64-bit processor) |
| **Desktop Window Manager** | Enabled (default on all supported Windows versions) |
| **Disk space** | ~5 MB |
| **Admin rights** | ❌ Not required |
| **Virtual Desktop isolation** | Windows 10 1607+ (build 14393) for IVirtualDesktopManager |

---

## Method 1 — Installer (Recommended)

The installer handles everything automatically and is the easiest way to get started.

### Steps

1. Download **StageManagerSetup.exe** from the [Releases page](../../releases).
2. Double-click `StageManagerSetup.exe`. No UAC prompt will appear.
3. The installer will:
   - Copy `stage_manager.exe` to `%LOCALAPPDATA%\StageManager\`
   - Add a **Start Menu** shortcut under *Stage Manager for Windows*
   - Register the application in **Windows Add/Remove Programs** (Settings → Apps → Installed Apps)
   - Add a startup registry entry so Stage Manager launches at Windows boot
4. Stage Manager starts automatically after installation.

### Silent (Unattended) Install

```powershell
StageManagerSetup.exe --silent
```

No UI is shown; the installer exits silently when complete. Useful for scripted deployments.

---

## Method 2 — Portable (No Install)

If you prefer not to install, you can run Stage Manager directly without touching the registry or any system folders.

1. Download **stage_manager.exe** from the [Releases page](../../releases).
2. Place it anywhere you like (e.g., `C:\Tools\StageManager\`).
3. Double-click `stage_manager.exe` to launch.

> [!NOTE]
> In portable mode, Stage Manager does **not** register itself to run at startup. You can enable this manually from the tray icon's right-click menu once it is running.

---

## Method 3 — Build from Source

See [docs/DEVELOPMENT.md](DEVELOPMENT.md) for a full guide covering prerequisites, build commands, and how to produce both binaries.

---

## Verifying It's Running

After installation or launch, look for the **Stage Manager squircle icon** in the system tray (bottom-right corner of the taskbar, in the notification area). If you don't see it, click the **^** arrow to show hidden tray icons.

**Keyboard shortcuts to confirm it's active:**

| Shortcut | Action |
|---|---|
| `Win + `` ` | Toggle the Stage Manager sidebar |
| `Win + Alt + ←` | Cycle to the previous stage |
| `Win + Alt + →` | Cycle to the next stage |

---

## Uninstalling

### Via Windows Settings (Recommended)

1. Open **Settings → Apps → Installed Apps**.
2. Search for **Stage Manager**.
3. Click the **⋯** menu → **Uninstall**.

### Via Command Line

```powershell
StageManagerSetup.exe --uninstall
```

This removes the installed files, Start Menu shortcut, and Windows uninstaller registration. It does **not** delete your configuration.

---

## Managing Startup Behavior

Stage Manager registers itself to start automatically with Windows using this registry key:

```
HKCU\Software\Microsoft\Windows\CurrentVersion\Run\StageManager
```

### Toggle via Tray Menu

Right-click the Stage Manager tray icon → **Run at Windows Startup** (checkmark = enabled).

### Toggle Manually

**To disable:**
```powershell
Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "StageManager"
```

**To re-enable:**
```powershell
$exe = "$env:LOCALAPPDATA\StageManager\stage_manager.exe"
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "StageManager" -Value $exe
```

---

## After Installation: What to Explore

- **Right-click the tray icon** to access all configuration options: sidebar edge (Left/Right), trigger mode, glass theme, grouping mode, backdrop blur, and startup toggle.
- See [docs/TROUBLESHOOTING.md](TROUBLESHOOTING.md) if you run into any issues.
- See [docs/CONFIGURATION.md](CONFIGURATION.md) for a full reference of all settings.
