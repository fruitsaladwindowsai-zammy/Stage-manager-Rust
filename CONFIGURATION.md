# Configuration Reference

Stage Manager for Windows provides deep customization over positioning, trigger sensitivity, grouping behaviors, glass visual themes, and spring animation dynamics.

---

## Table of Contents
- [Quick Configuration via System Tray](#quick-configuration-via-system-tray)
- [Configuration Fields Reference](#configuration-fields-reference)
- [Glass Themes (Windows 11 Acrylic & Mica)](#glass-themes-windows-11-acrylic--mica)
- [Grouping Modes](#grouping-modes)
- [DPI Awareness & Measurement Units](#dpi-awareness--measurement-units)
- [Windows Startup Registry Key](#windows-startup-registry-key)
- [Planned Configuration File Format (`config.toml`)](#planned-configuration-file-format-configtoml)

---

## Quick Configuration via System Tray

The fastest way to configure Stage Manager is right-clicking the **Stage Manager squircle icon** in your Windows system tray:

```
+-----------------------------------+
| Side Edge            ▶  (•) Left  |
|                         ( ) Right |
| Trigger Mode         ▶  (•) Full Edge
|                         ( ) Top Corner
|                         ( ) Bottom Corner
| [ ] Backdrop Blur Strip           |
| Glass Theme          ▶  (•) Dark Frosted
|                         ( ) Light Frosted
|                         ( ) Acrylic
|                         ( ) Mica
| Grouping             ▶  (•) By App
|                         ( ) Single Window
|-----------------------------------|
| [✓] Run at Windows Startup        |
|-----------------------------------|
| Exit Stage Manager                |
+-----------------------------------+
```

All tray settings apply immediately without requiring a restart.

---

## Configuration Fields Reference

The table below outlines every field in the runtime `Config` struct (`src/config.rs`):

| Field | Rust Type | Default | Description | Valid Values |
|---|---|---|---|---|
| `sidebar_edge` | `SidebarEdge` | `SidebarEdge::Left` | Which side of the monitor work area the strip docks to. | `Left`, `Right` |
| `trigger_mode` | `TriggerMode` | `TriggerMode::FullEdge` | Mouse trigger sensor location for revealing the strip. | `FullEdge`, `TopCorner`, `BottomCorner` |
| `grouping_mode` | `GroupingMode` | `GroupingMode::ByApplication` | How open windows are clustered into stages. | `ByApplication`, `SingleWindow` |
| `glass_theme` | `GlassTheme` | `GlassTheme::DarkFrosted` | Visual appearance and blur backdrop styling. | `DarkFrosted`, `LightFrosted`, `Acrylic`, `Mica` |
| `auto_hide` | `bool` | `true` | Automatically retracts the sidebar off-screen when inactive. | `true`, `false` |
| `blur_backdrop` | `bool` | `false` | Shows a Gaussian frosted translucent backdrop strip behind thumbnails. | `true`, `false` |
| `include_active_app` | `bool` | `false` | Whether to display the currently active/focused stage in the strip. | `true`, `false` |
| `sidebar_width` | `i32` | `224` | Logical strip width in 96-DPI pixels. Scales automatically per-monitor. | `160` to `360` |
| `edge_margin` | `i32` | `22` | Outer gap from the screen edge to floating stage preview cards. | `10` to `60` |
| `stage_gap` | `i32` | `18` | Vertical spacing between stages in the carousel ribbon. | `8` to `40` |
| `max_previews_per_stage` | `usize` | `5` | Maximum number of cascaded preview cards stacked per stage group. | `1` to `8` |
| `toggle_hotkey_vk` | `u32` | `0xC0` | Virtual key code for `Win + <Key>` toggle (`0xC0` = backtick). | Any Win32 `VK_*` code |
| `transition_overlay_enabled` | `bool` | `true` | Enables animated DWM swoop transition on stage switch. | `true`, `false` |

---

## Glass Themes (Windows 11 Acrylic & Mica)

Stage Manager supports 4 distinct rendering styles for the backdrop and window overlays (`src/blur.rs`):

1. **`DarkFrosted` (Default):** Deep dark translucent glass (`rgba(18, 18, 24, 0.72)`) matching macOS dark mode.
2. **`LightFrosted`:** Translucent light glass (`rgba(245, 245, 250, 0.78)`) with dark text and borders.
3. **`Acrylic`:** True Windows 11 Acrylic material leveraging `DWMWA_SYSTEMBACKDROP_TYPE = 3`.
4. **`Mica`:** Native Windows 11 Mica material matching system theme surface colors (`DWMWA_SYSTEMBACKDROP_TYPE = 2`).

---

## Grouping Modes

### 1. `ByApplication` (Default)
- Windows sharing the same process executable (e.g. 3 Google Chrome windows) are merged into a single stage card.
- Stacks up to `max_previews_per_stage` (5) live cascaded thumbnails.
- **Hover Fan-Out:** Hovering over the stage smoothly fans out all grouped windows horizontally, with individual squircle icons for direct selection.

### 2. `SingleWindow`
- Every open top-level window forms its own dedicated stage card.
- Windows are never automatically grouped unless manually merged using `Shift + Click`.

---

## DPI Awareness & Measurement Units

Stage Manager is fully **Per-Monitor V2 DPI Aware** (`DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2`).

- All coordinates and dimensions in `Config` are specified in **logical pixels at 96 DPI**.
- When rendering on high-DPI displays (e.g. 4K at 150% or 200%), Stage Manager queries `GetDpiForMonitor` and dynamically scales thumbnails, margins, and Direct2D vector graphics:
  $$\text{Physical Pixels} = \text{Logical Pixels} \times \frac{\text{Monitor DPI}}{96}$$

---

## Windows Startup Registry Key

To start Stage Manager automatically when logging into Windows, the application sets the following user-level registry value:

- **Key:** `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`
- **Value Name:** `StageManager`
- **Value Type:** `REG_SZ`
- **Value Data:** `"C:\Users\<Username>\AppData\Local\StageManager\stage_manager.exe"`

---

## Planned Configuration File Format (`config.toml`)

Future builds will support a persistent configuration file at `%APPDATA%\StageManager\config.toml`. See [stagemanager.example.toml](stagemanager.example.toml) for the complete reference file.
