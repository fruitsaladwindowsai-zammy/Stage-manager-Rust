# Keybindings & Input Reference

Stage Manager for Windows provides smooth, seamless keyboard shortcuts, mouse edge sensors, and click modifiers for rapid multitasking.

Global keyboard shortcuts are registered through the Win32 `RegisterHotKey` API with automatic fallback mechanisms to prevent conflicts with existing system hotkeys.

---

## Table of Contents
- [Default Keyboard Shortcuts](#default-keyboard-shortcuts)
- [Mouse & Pointer Triggers](#mouse--pointer-triggers)
- [Modifier Clicks & Window Merging](#modifier-clicks--window-merging)
- [Carousel & Scroll Controls](#carousel--scroll-controls)
- [System Tray Interaction](#system-tray-interaction)
- [Resolving Hotkey Conflicts](#resolving-hotkey-conflicts)
- [Customizing Keybindings in Code](#customizing-keybindings-in-code)

---

## Default Keyboard Shortcuts

| Shortcut | Action | Scope | Description |
|---|---|---|---|
| `Win + \`` *(backtick)* | `toggle_sidebar` | Global | Toggles Stage Manager sidebar strip visibility on the active monitor. |
| `Alt + \`` *(backtick)* | `toggle_sidebar` (fallback) | Global | Automatic fallback if another application reserves `Win + \``. |
| `Win + Alt + →` | `cycle_next_stage` | Active Monitor | Smoothly transitions to the next stage down the strip. |
| `Win + Alt + ←` | `cycle_prev_stage` | Active Monitor | Smoothly transitions to the previous stage up the strip. |
| `Win + Ctrl + →` | Windows Virtual Desktop Next | OS Global | Switches to next Virtual Desktop; Stage Manager isolates workspaces automatically. |
| `Win + Ctrl + ←` | Windows Virtual Desktop Prev | OS Global | Switches to previous Virtual Desktop; restores that desktop's stages. |

---

## Mouse & Pointer Triggers

Stage Manager uses low-level pointer sensors (`WH_MOUSE_LL`) to detect when your cursor approaches the screen edge.

### 1. Edge Reveal Modes (Configurable via Tray)

| Trigger Mode | Activation Area | Behavior |
|---|---|---|
| **Full Side Edge** *(Default)* | Entire left or right screen border (0–12 logical px) | Moving the mouse anywhere to the active edge slides out the sidebar instantly. |
| **Top Corner** | Top 18×18 logical px corner sensor | Moving into the top corner reveals the strip; prevents accidental triggers when clicking near window buttons. |
| **Bottom Corner** | Bottom 18×18 logical px corner sensor | Moving into the bottom corner reveals the strip. |

### 2. Auto-Hide & Dismissal
- When your cursor leaves the sidebar strip, a **260ms grace timer** starts.
- If the cursor does not re-enter, the sidebar smoothly retracts off-screen via damped spring physics.
- Clicking any blank area on your desktop or focusing another window immediately slides the strip away.

---

## Modifier Clicks & Window Merging

| Action | Input | Description |
|---|---|---|
| **Switch Stage** | `Left Click` on Stage Card | Minimizes / hides current stage and brings the selected stage to the front. |
| **Pick Specific Grouped Window** | `Hover` then `Left Click` | Hovering over a multi-window stage expands all windows horizontally. Click any specific window to raise it directly. |
| **Merge Stage into Current Workspace** | `Shift + Left Click` on Stage Card | Pulls the clicked application into your active stage so they are grouped together side-by-side. |
| **Separate Window from Stage** | Drag titlebar out of group | Moving a window away from the active group splits it back into its own independent stage. |

---

## Carousel & Scroll Controls

When you have more open stages than can fit vertically on your screen, Stage Manager enters **Infinite Carousel Mode**:

| Input | Target Area | Result |
|---|---|---|
| **Mouse Wheel Up** | Over Sidebar Strip | Scrolls the stage ribbon upward. |
| **Mouse Wheel Down** | Over Sidebar Strip | Scrolls the stage ribbon downward. |
| **Hover Top Endpoint** | Top 40px of screen edge | Smoothly auto-scrolls the ribbon upward with infinite looping. |
| **Hover Bottom Endpoint** | Bottom 40px of screen edge | Smoothly auto-scrolls the ribbon downward with infinite looping. |

> [!NOTE]
> The carousel wraps indefinitely in both directions. The last stage seamlessly transitions into the first stage with no visual jump or hard seam.

---

## System Tray Interaction

The Stage Manager notification icon resides in your Windows system tray (bottom-right next to the clock):

| Mouse Action | Effect |
|---|---|
| **Single Left Click** | Toggles sidebar visibility on the primary monitor. |
| **Right Click** | Opens the Stage Manager Settings Context Menu: |
| | • **Side Edge:** Switch between *Left* and *Right* screen borders |
| | • **Trigger Mode:** *Full Edge*, *Top Corner*, or *Bottom Corner* |
| | • **Backdrop Blur:** Toggle Gaussian frosted blur strip |
| | • **Glass Theme:** *Dark Frosted*, *Light Frosted*, *Acrylic*, or *Mica* |
| | • **Grouping:** *By Application* or *Single Window* |
| | • **Run at Windows Startup:** Enable/disable boot launch |
| | • **Exit Stage Manager:** Cleanly unregisters hooks and exits |

---

## Resolving Hotkey Conflicts

If `Win + \`` does not respond:
1. Another utility (such as terminal dropdowns, Quake consoles, or AMD/NVIDIA overlays) may have bound `Win + \`` or `VK_OEM_3`.
2. Stage Manager will automatically register `Alt + \`` as a backup.
3. You can still reveal Stage Manager anytime by sliding your cursor to the screen edge or clicking the system tray icon.

---

## Customizing Keybindings in Code

Keyboard shortcuts are defined in `src/hotkeys.rs`. The toggle key code is defined in `src/config.rs`:

```rust
// In Config::default()
pub toggle_hotkey_vk: u32 = 0xC0; // VK_OEM_3 (~ / ` key)
```

Common virtual key codes:
- `0xC0` (`VK_OEM_3`): Backtick / Tilde key (`~` / `` ` ``)
- `0x09` (`VK_TAB`): Tab key
- `0x20` (`VK_SPACE`): Spacebar
- `0x51` (`VK_Q`): Q key
