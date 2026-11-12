# Configuration Reference

Stage Manager for Windows is configured entirely at **runtime** via the **system tray icon**. There is no configuration file yet — all settings are held in memory and reset to defaults on restart (a persistent config file is planned).

---

## How to Configure

1. Find the **Stage Manager icon** in the Windows system tray (bottom-right of the taskbar — you may need to click the `^` arrow to show hidden icons).
2. **Right-click** the tray icon to open the context menu.
3. Select the option you want to change. Settings take effect immediately.

---

## Settings Reference

| Field | Type | Default | Description | Valid Values |
|-------|------|---------|-------------|--------------|
| `sidebar_edge` | `SidebarEdge` | `Left` | Which screen edge the sidebar appears on | `Left`, `Right` |
| `trigger_mode` | `TriggerMode` | `FullEdge` | How the sidebar is revealed | `FullEdge`, `TopCorner`, `BottomCorner` |
| `grouping_mode` | `GroupingMode` | `ByApplication` | How windows are grouped in the sidebar | `ByApplication`, `SingleWindow` |
| `glass_theme` | `GlassTheme` | `DarkFrosted` | Visual style of the sidebar | `DarkFrosted` |
| `auto_hide` | `bool` | `true` | Sidebar hides automatically when cursor leaves | `true`, `false` |
| `blur_backdrop` | `bool` | `false` | Enables blurred/frosted glass backdrop | `true`, `false` |
| `include_active_app` | `bool` | `false` | Shows the currently active app in the sidebar | `true`, `false` |
| `sidebar_width` | `i32` | `224` | Width of the sidebar strip in logical pixels (96 DPI baseline) | Integer (logical px) |
| `edge_margin` | `i32` | `22` | Gap between the sidebar and the screen edge in logical pixels | Integer (logical px) |
| `stage_gap` | `i32` | `18` | Gap between stage groups in logical pixels | Integer (logical px) |
| `max_previews_per_stage` | `usize` | `5` | Maximum window previews shown per stage group | Positive integer |
| `toggle_hotkey_vk` | `u32` | `0xC0` | Virtual key code for the sidebar toggle hotkey (Win+key) | VK_* code (see table below) |
| `transition_overlay_enabled` | `bool` | `true` | Enables the Direct2D squircle icon overlay during transitions | `true`, `false` |

---

## Tray Menu → Config Field Mapping

| Tray Menu Item | Config Field(s) Changed |
|----------------|------------------------|
| **Sidebar Edge → Left** | `sidebar_edge = Left` |
| **Sidebar Edge → Right** | `sidebar_edge = Right` |
| **Trigger → Full Edge** | `trigger_mode = FullEdge` |
| **Trigger → Top Corner** | `trigger_mode = TopCorner` |
| **Trigger → Bottom Corner** | `trigger_mode = BottomCorner` |
| **Blur Backdrop** (toggle) | `blur_backdrop` toggled |
| **Theme → Dark Frosted** | `glass_theme = DarkFrosted` |
| **Grouping → By Application** | `grouping_mode = ByApplication` |
| **Grouping → Single Window** | `grouping_mode = SingleWindow` |
| **Run at Startup** (toggle) | Writes/removes registry key (see below) |

> [!NOTE]
> `auto_hide`, `include_active_app`, `sidebar_width`, `edge_margin`, `stage_gap`, `max_previews_per_stage`, `toggle_hotkey_vk`, and `transition_overlay_enabled` are currently only configurable by editing source defaults and recompiling. They will be exposed in the future config file.

---

## Registry Startup Entry

When **Run at Startup** is enabled from the tray menu, Stage Manager writes a registry value so it launches automatically when you log in:

| Property | Value |
|----------|-------|
| **Key path** | `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` |
| **Value name** | `StageManager` |
| **Value type** | `REG_SZ` |
| **Value data** | Full path to `stage_manager.exe` (e.g. `C:\Users\<user>\AppData\Local\StageManager\stage_manager.exe`) |

To verify or remove the entry manually, open **Registry Editor** (`regedit.exe`) and navigate to the key above, or run in PowerShell:

```powershell
# Check current value
Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "StageManager" -ErrorAction SilentlyContinue

# Remove manually (if needed)
Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "StageManager" -ErrorAction SilentlyContinue
```

> [!TIP]
> Toggling **Run at Startup** from the tray menu is the recommended way to manage the startup entry. Manual registry edits are only needed for troubleshooting.

---

## Hotkey VK Codes (`toggle_hotkey_vk`)

The toggle hotkey is **Win + `toggle_hotkey_vk`**. The default is <kbd>Win</kbd>+<kbd>`</kbd> (backtick, `0xC0`).

To change it, modify the `toggle_hotkey_vk` field in `src/config.rs` and recompile.

**Common VK_* values:**

| Key | VK Name | Hex Code |
|-----|---------|----------|
| `` ` `` (backtick / grave) | `VK_OEM_3` | `0xC0` *(default)* |
| `Space` | `VK_SPACE` | `0x20` |
| `Tab` | `VK_TAB` | `0x09` |
| `F1` | `VK_F1` | `0x70` |
| `F2` | `VK_F2` | `0x71` |
| `F3` | `VK_F3` | `0x72` |
| `F4` | `VK_F4` | `0x73` |
| `F5` | `VK_F5` | `0x74` |
| `F6` | `VK_F6` | `0x75` |
| `F7` | `VK_F7` | `0x76` |
| `F8` | `VK_F8` | `0x77` |
| `F9` | `VK_F9` | `0x78` |
| `F10` | `VK_F10` | `0x79` |
| `F11` | `VK_F11` | `0x7A` |
| `F12` | `VK_F12` | `0x7B` |
| `0`–`9` (number row) | `VK_0`–`VK_9` | `0x30`–`0x39` |
| `A`–`Z` | `VK_A`–`VK_Z` | `0x41`–`0x5A` |
| `;` | `VK_OEM_1` | `0xBA` |
| `=` | `VK_OEM_PLUS` | `0xBB` |
| `,` | `VK_OEM_COMMA` | `0xBC` |
| `-` | `VK_OEM_MINUS` | `0xBD` |
| `.` | `VK_OEM_PERIOD` | `0xBE` |
| `/` | `VK_OEM_2` | `0xBF` |
| `[` | `VK_OEM_4` | `0xDB` |
| `\` | `VK_OEM_5` | `0xDC` |
| `]` | `VK_OEM_6` | `0xDD` |
| `'` | `VK_OEM_7` | `0xDE` |

> [!NOTE]
> For a full reference, see [Virtual-Key Codes on MSDN](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes).

---

## DPI Scaling Notes

All pixel-based config fields (`sidebar_width`, `edge_margin`, `stage_gap`) are specified in **logical pixels at 96 DPI** (100% scale). Stage Manager scales these values automatically for each monitor's DPI setting at runtime:

- At **125% DPI** (120 DPI): values are multiplied by `1.25`
- At **150% DPI** (144 DPI): values are multiplied by `1.50`
- At **200% DPI** (192 DPI): values are multiplied by `2.00`

For multi-monitor setups with different DPI scales, each monitor's sidebar is scaled independently.

---

## Future: Persistent Config File

> [!IMPORTANT]
> Currently, all settings revert to defaults on restart. A persistent config file is planned for a future release.

The planned config file location is:

```
%APPDATA%\StageManager\config.toml
```

Which expands to something like:

```
C:\Users\<YourUsername>\AppData\Roaming\StageManager\config.toml
```

The planned TOML format will look like:

```toml
sidebar_edge = "Left"
trigger_mode = "FullEdge"
grouping_mode = "ByApplication"
glass_theme = "DarkFrosted"
auto_hide = true
blur_backdrop = false
include_active_app = false
sidebar_width = 224
edge_margin = 22
stage_gap = 18
max_previews_per_stage = 5
toggle_hotkey_vk = 0xC0
transition_overlay_enabled = true
```

> [!TIP]
> To be notified when persistent config lands, watch the [CHANGELOG](../CHANGELOG.md) or subscribe to releases on GitHub.
