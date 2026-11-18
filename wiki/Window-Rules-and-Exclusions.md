# Window Rules & Exclusions

Stage Manager filters out background processes, shell overlays, and system utilities so that only genuine application windows appear on your stages.

---

## 1. Automatic Built-In Filters

During window enumeration (`EnumWindows` in `src/window_tracker.rs`), the following checks are applied:

1. **Visibility & Cloaking:**
   - Must have `WS_VISIBLE` style.
   - Must not be cloaked by Windows DWM (`DWMWA_CLOAKED`).
2. **Window Style Attributes:**
   - Must have `WS_CAPTION` or standard application window styles.
   - Tooltips, popup menus, system dropdowns, and transparent tool windows (`WS_EX_TOOLWINDOW`) are excluded.
3. **Internal App Exclusion:**
   - Stage Manager's own host, sidebar, and layered overlay windows are filtered out by Process ID (PID).
4. **Shell & Desktop Components:**
   - Windows Taskbar (`Shell_TrayWnd`), Program Manager (`Progman`), and wallpaper workers (`WorkerW`) are excluded.

---

## 2. Common Filtered Executables

System background hosts are filtered out by default:
- `Taskmgr.exe` (Windows Task Manager)
- `SnippingTool.exe` / `ScreenClippingHost.exe`
- `ShellExperienceHost.exe`
- `StartMenuExperienceHost.exe`
- `SearchApp.exe`
- `LockApp.exe`
