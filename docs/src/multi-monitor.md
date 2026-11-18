# Multi-Monitor & Extended Displays

Stage Manager treats every display and virtual desktop as an independent, isolated multitasking workspace.

---

## 1. Per-Monitor Strip Isolation

Unlike taskbars that mirror or duplicate cards across screens, Stage Manager implements true **Per-Monitor Isolation**:

- Each connected monitor runs its own independent `Sidebar` window and trigger zone.
- Windows positioned on Monitor 1 are managed exclusively on Monitor 1's stage strip.
- Windows positioned on Monitor 2 are managed exclusively on Monitor 2's stage strip.
- Moving an application window from Monitor 1 to Monitor 2 automatically migrates its stage card across to the destination monitor.

---

## 2. Display Hotplugging

When displays are connected, disconnected, or reconfigured:
1. Windows emits `WM_DISPLAYCHANGE`.
2. Stage Manager re-enumerates displays via `EnumDisplayMonitors`.
3. New monitors spawn independent sidebars; disconnected sidebars are torn down gracefully.
4. Window associations are retained without crashing.

---

## 3. Windows Virtual Desktops

Stage Manager integrates with the Windows 10 & 11 `IVirtualDesktopManager` COM interface:
- Windows are tagged with their owning Virtual Desktop GUID via `GetWindowDesktopId`.
- Switching desktops (`Win + Ctrl + Left/Right`) swaps the active stage collection immediately.
- A clean virtual desktop provides a clean stage ribbon with zero window bleed from prior desktops.
