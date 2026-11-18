# Configuration Guide

Stage Manager for Windows provides fine-grained control over positioning, sensitivity, visual glass styling, and grouping mechanics.

---

## 1. System Tray Menu Configuration

Right-clicking the **Stage Manager squircle icon** in your Windows system tray exposes the runtime configuration menu:

- **Side Edge:** Dock the sidebar to the *Left* or *Right* screen boundary.
- **Trigger Mode:**
  - *Full Edge (Default):* Trigger anywhere along the active screen edge.
  - *Top Corner:* Trigger via an 18×18px sensor at the top-left or top-right corner.
  - *Bottom Corner:* Trigger via an 18×18px sensor at the bottom-left or bottom-right corner.
- **Backdrop Blur Strip:** Toggle translucent frosted backdrop behind the floating cards.
- **Glass Theme:**
  - *Dark Frosted:* Deep translucent dark glass (`rgba(18, 18, 24, 0.72)`).
  - *Light Frosted:* Bright translucent glass (`rgba(245, 245, 250, 0.78)`).
  - *Acrylic:* Windows 11 Acrylic material blur (`DWMWA_SYSTEMBACKDROP_TYPE = 3`).
  - *Mica:* Windows 11 Mica surface material blur (`DWMWA_SYSTEMBACKDROP_TYPE = 2`).
- **Grouping:** Switch between *By Application* (cascaded stacks with hover fan-out) and *Single Window* (standalone cards).
- **Run at Windows Startup:** Automatically launch on Windows boot via user registry key.

---

## 2. Windows Startup Registry

Stage Manager manages startup cleanly in the current user's registry hive without requiring administrator privileges:

```
HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run\StageManager
```

Value: `"C:\Users\<Username>\AppData\Local\StageManager\stage_manager.exe"`

---

## 3. High-DPI Scaling

All pixel dimensions in configuration structures are specified in **logical pixels at 96 DPI**. On high-DPI displays (125%, 150%, 200%), Stage Manager queries `GetDpiForMonitor` and scales thumbnail coordinates, margins, and vector icons automatically.
