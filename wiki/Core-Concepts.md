# Core Concepts

Understanding the core design principles of Stage Manager will help you get the most out of your multitasking workflow.

---

## 1. What is a "Stage"?

Traditional Windows taskbars represent each individual window as an isolated button or grouped list. In contrast, **Stage Manager organizes your work into context-rich Stages**:

- A **Stage** is an active workspace consisting of one or more windows that belong together.
- For example, a research stage might contain a browser window and a text editor side-by-side.
- Inactive stages float in an auto-hiding ribbon on the side of your display, showing real-time, hardware-rendered thumbnail previews.

---

## 2. Active Stage vs. Inactive Stages

At any given moment on a monitor:
- Exactly **one stage is active** and displayed in the main work area.
- All other stages are **inactive** and parked along the Stage Manager ribbon.
- Clicking an inactive stage seamlessly swaps it into focus while retracting the previously active stage back onto the strip.

---

## 3. Window Grouping Models

Stage Manager offers two distinct grouping models (configurable via the system tray):

### ByApplication (Default)
Windows belonging to the same executable process (e.g. three Visual Studio Code instances or multiple Chrome windows) automatically cluster into a single stage card.
- The stage card displays up to 5 cascaded preview thumbnails.
- **Horizontal Fan-Out:** Hovering over the card fans out the windows horizontally with individual squircle icons for direct window targeting.

### SingleWindow
Every open top-level window forms its own standalone stage. Windows are only grouped if manually combined using `Shift + Click`.

---

## 4. Merging and Separating Windows

You can dynamically construct multi-app stages on the fly:

- **Merge:** Hold `Shift` and left-click an inactive stage card. It will immediately join your current active stage rather than replacing it.
- **Separate:** Drag any window titlebar away from the grouped layout, and Stage Manager will split it back into an independent stage.

---

## 5. Workspaces and Multi-Screen Matrix

Stage Manager organizes its state as a 2D matrix keyed by:
$$\text{Workspace} = (\text{VirtualDesktopId}, \text{MonitorHandle})$$

- Windows on Display 1 never bleed into Display 2's stage ribbon.
- Switching Windows Virtual Desktops (`Win + Ctrl + Left/Right`) completely swaps the stage ribbon to match the newly active virtual desktop.
