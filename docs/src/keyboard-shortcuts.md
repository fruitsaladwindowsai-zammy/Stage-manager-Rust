# Keyboard Shortcuts & Controls

Stage Manager supports global keyboard shortcuts, modifier clicks, and mouse edge triggers for fast multitasking.

---

## Global Shortcuts

| Shortcut | Action | Description |
|---|---|---|
| `Win + \`` *(backtick)* | `toggle_sidebar` | Slides the Stage Manager sidebar in or out. |
| `Alt + \`` *(backtick)* | `toggle_sidebar` (fallback) | Automatic fallback if another utility reserves `Win + \``. |
| `Win + Alt + →` | `cycle_next_stage` | Smoothly transitions to the next stage down the strip. |
| `Win + Alt + ←` | `cycle_prev_stage` | Smoothly transitions to the previous stage up the strip. |
| `Win + Ctrl + →` | Next Virtual Desktop | Switches to next Virtual Desktop; Stage Manager isolates workspaces automatically. |
| `Win + Ctrl + ←` | Previous Virtual Desktop | Switches to previous Virtual Desktop; restores that desktop's stages. |

---

## Mouse & Pointer Triggers

| Input | Target Area | Result |
|---|---|---|
| **Hover Screen Edge** | Left or Right Screen Border | Smoothly slides out the stage strip via spring physics. |
| **Left Click** | Stage Card | Minimizes previous stage and activates selected stage. |
| **Shift + Left Click** | Stage Card | Merges the clicked application into your active stage. |
| **Hover** | Multi-Window Group | Horizontally fans out all windows in the group. |
| **Mouse Wheel** | Over Sidebar Strip | Scrolls the stage carousel vertically. |
| **Hover Endpoint** | Top or Bottom 40px of edge | Initiates continuous infinite scrolling. |
