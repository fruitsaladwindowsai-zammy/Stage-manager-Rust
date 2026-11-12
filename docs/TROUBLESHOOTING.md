# Troubleshooting

Common issues and their solutions. If your problem isn't listed here, please [open a GitHub issue](../../issues/new/choose).

---

## Tray Icon Not Showing After Explorer Crash

**Symptom:** The Stage Manager icon disappears from the system tray after Windows Explorer crashes or restarts.

**Cause:** This is expected Windows behaviour. Stage Manager listens for the `TaskbarCreated` (`WM_TASKBARCREATED`) broadcast message that Explorer sends when it restarts, and automatically re-registers the tray icon.

**Fix:** Wait a moment after Explorer restarts — the icon should reappear automatically. If it does not, restart Stage Manager manually:

```powershell
Start-Process "$env:LOCALAPPDATA\StageManager\stage_manager.exe"
```

Or, if running from source: re-run `cargo run --bin stage_manager`.

---

## Black Flicker on First Sidebar Reveal

**Symptom:** A brief black flash or flicker when the sidebar appears for the first time after launch.

**Cause:** This is a **normal and expected** DWM thumbnail registration timing artefact. The first call to `DwmRegisterThumbnail` requires DWM to initialise the thumbnail handle, which introduces a one-frame delay.

**Fix:** None required. The flicker only occurs once per session (on first reveal). It disappears entirely after the initial reveal.

---

## Hotkey `Win+`` ` Conflicts With Another App

**Symptom:** Pressing `Win+`` ` does nothing, or triggers a different application.

**Cause:** Another running application has already registered the `VK_OEM_3` global hotkey (the backtick/grave key). Windows global hotkeys are first-come, first-served.

**Fix:** Stage Manager automatically falls back to `Alt+`` ` when the primary hotkey is unavailable. Use `Alt+`` ` to toggle the sidebar. To use `Win+`` ` exclusively, identify and close or reconfigure the conflicting application (common culprits: terminal emulators, gaming overlays).

---

## Sidebar Appears on the Wrong Monitor

**Symptom:** On a multi-monitor setup, the sidebar appears on a monitor you didn't expect.

**Cause:** Stage Manager runs **independent sidebars on each connected monitor**. The sidebar that responds is the one associated with the monitor where the focused window resides.

**Fix:**
1. Right-click the tray icon and check the **sidebar_edge** setting to verify which edge is configured.
2. Move focus to a window on the desired monitor, then trigger the sidebar.
3. Each monitor's sidebar is independent — interacting with windows on a given monitor controls that monitor's sidebar.

---

## Windows from Other Virtual Desktops Are Appearing

**Symptom:** Stage Manager shows windows that belong to a different Windows Virtual Desktop.

**Cause:** Virtual Desktop isolation requires `IVirtualDesktopManager`, which was introduced in **Windows 10 version 1607 (build 14393)**. On older builds the isolation interface is unavailable.

**Fix:** Ensure you are running **Windows 10 version 1607 or later** (or any version of Windows 11). Check your build:

```powershell
(Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion").CurrentBuildNumber
```

The result must be `14393` or higher.

---

## App Won't Start — Single Instance Guard

**Symptom:** Double-clicking `stage_manager.exe` does nothing, or a second window never appears.

**Cause:** Stage Manager uses `CreateMutexW` to enforce a single running instance. If a previous instance is still running (even invisibly), the new launch exits immediately.

**Fix:** Open **Task Manager** (`Ctrl+Shift+Esc`) → **Details** tab, look for `stage_manager.exe`, right-click → **End task**. Then relaunch.

Or via PowerShell:

```powershell
Stop-Process -Name stage_manager -Force
```

---

## Access Denied During `cargo build --release`

**Symptom:** The Rust build fails with an access-denied or file-locked error on `stage_manager.exe`.

**Cause:** A running `stage_manager.exe` holds an exclusive lock on its own binary, preventing the linker from overwriting it.

**Fix:** Stop the running instance before building:

```powershell
Stop-Process -Name stage_manager -Force
cargo build --release
```

---

## Grouping Too Aggressive

**Symptom:** Too many windows are being grouped into a single stage tile, making individual windows hard to access.

**Fix:** Right-click the tray icon → **Grouping** → switch to **Single Window** mode. In this mode each window gets its own stage tile regardless of which application it belongs to.

---

## Sidebar Won't Auto-Hide

**Symptom:** The sidebar remains permanently visible instead of hiding when your cursor leaves it.

**Fix:** Right-click the tray icon and confirm **auto_hide** is enabled (checkmark present). If it is already checked, try toggling it off and back on.

> [!NOTE]
> Auto-hide is per-monitor. If you have multiple monitors, verify the setting for the specific monitor's sidebar you are troubleshooting.

---

## DPI / Scaling Issues on 4K or High-DPI Monitors

**Symptom:** The sidebar, icons, or thumbnails appear too small or too large on a high-DPI monitor.

**Cause:** All internal size values in Stage Manager are **logical 96-DPI units**. The app queries the actual per-monitor DPI at runtime via `GetDpiForMonitor` and scales accordingly.

**Fix:** Ensure the monitor DPI is set correctly in Windows Display Settings:

1. Right-click the Desktop → **Display settings**.
2. Select the affected monitor.
3. Under **Scale**, choose the recommended percentage (or adjust to preference).
4. Restart Stage Manager after changing scale settings.

If issues persist after a DPI change, fully quit Stage Manager (tray → **Exit**) and relaunch.

---

## Still Stuck?

- Search [existing issues](../../issues) — your problem may already have a workaround.
- Open a [new Bug Report](../../issues/new?template=bug_report.md) with your Windows version, DPI scale, monitor count, and GPU model.
