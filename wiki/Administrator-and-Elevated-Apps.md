# Administrator & Elevated Applications

This guide explains how Stage Manager interacts with elevated (Administrator) windows under Windows User Account Control (UAC).

---

## 1. User Interface Privilege Isolation (UIPI)

Windows enforces **UIPI (User Interface Privilege Isolation)**:
- A non-elevated (standard user) process cannot send certain window messages (`WM_COMMAND`, `WM_SYSCOMMAND`) or manipulate windows owned by an elevated (Administrator) process.
- **Stage Manager runs as standard user by default**, requiring zero administrator privileges to install, configure, or run.

---

## 2. Managing Elevated Windows

When an application runs with elevated privileges (e.g. an Administrator Command Prompt or PowerShell):
- Stage Manager can still display live DWM thumbnails and track window boundaries.
- However, bringing an elevated window to the front or switching focus via Win32 `SetForegroundWindow` may be restricted by Windows security policies.

---

## 3. Running Stage Manager Elevated (Optional)

If your daily workflow involves managing numerous elevated development tools or administrative terminals, you can optionally run Stage Manager as Administrator:

1. Right-click `stage_manager.exe` (or your Start Menu shortcut).
2. Select **Run as administrator**.
3. Stage Manager will now possess matching integrity level to manage all elevated windows without restriction.
