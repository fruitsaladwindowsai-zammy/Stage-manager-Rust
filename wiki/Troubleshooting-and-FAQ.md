# Troubleshooting & Frequently Asked Questions

---

## 1. Frequently Asked Questions

### Q: Why does Stage Manager not take screenshots like other tools?
**A:** Taking screenshots in loops burns CPU and causes stuttering during video playback or typing. Stage Manager uses the Windows DWM Thumbnail API (`DwmRegisterThumbnail`), letting the GPU compositor blit live window frames directly with zero CPU overhead.

### Q: Does Stage Manager require administrator rights?
**A:** No. Stage Manager installs to `%LOCALAPPDATA%\StageManager`, registers in the user registry hive (`HKCU`), and runs entirely within standard user privileges.

### Q: Does it work with multi-monitor setups?
**A:** Yes. Each connected display receives its own independent Stage Manager strip, DPI scaling, and mouse trigger zone.

---

## 2. Common Issues & Solutions

### 1. `Access is denied (os error 5)` during compilation
- **Cause:** `stage_manager.exe` is currently running and locking the binary file.
- **Fix:** Run `Stop-Process -Name stage_manager -Force` in PowerShell, then retry compilation.

### 2. PowerShell execution policy blocks `.ps1` scripts
- **Cause:** Windows defaults to `Restricted` script execution.
- **Fix:** Run scripts using:
  ```powershell
  powershell -ExecutionPolicy Bypass -File .\scripts\build-installer.ps1
  ```

### 3. Tray icon disappears after Explorer crash
- Stage Manager automatically listens for the `TaskbarCreated` Win32 message and re-registers its tray icon as soon as Explorer restarts.

### 4. Global hotkey `Win + \`` does not respond
- Another app may have bound `Win + \`` (`VK_OEM_3`). Stage Manager automatically registers `Alt + \`` as a fallback. You can also reveal the strip by moving the mouse to the screen edge.
