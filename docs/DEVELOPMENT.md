# Development Guide

This guide covers everything you need to build Stage Manager from source, understand its codebase, and contribute.

---

## Prerequisites

| Requirement | Details |
|-------------|---------|
| **OS** | Windows 10 (1903+) or Windows 11 |
| **Rust** | Stable toolchain (`rustup install stable`) |
| **Target** | `stable-x86_64-pc-windows-gnu` (recommended) — MSVC also works |
| **DWM** | Desktop Window Manager must be enabled (default on all supported Windows versions) |
| **Git** | For cloning the repository |

> [!NOTE]
> The GNU toolchain is recommended and what CI uses. MSVC (`stable-x86_64-pc-windows-msvc`) also compiles and runs correctly.

### Install the GNU target (if not already set up)

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
```

---

## Clone & Build

```powershell
# Clone the repository
git clone https://github.com/<your-org>/stage-manager-windows.git
cd "stage-manager-windows"

# Install dependencies and compile (debug build)
cargo build
```

### Run in development mode

```powershell
cargo run
```

This launches the tray app. Look for the Stage Manager icon in the system tray.

---

## Running Tests

```powershell
cargo test
```

All **12 tests** should pass on a clean build. No special environment is required.

```
running 12 tests
test ... ok
...
test result: ok. 12 passed; 0 failed; 0 ignored
```

---

## Building a Release

> [!CAUTION]
> You **must** kill any running `stage_manager.exe` before building a release. If the process is running, the linker cannot overwrite the binary and you will get **Access Denied (OS error 5)**.

```powershell
# Kill the running process first (safe to run even if not running)
Stop-Process -Name stage_manager -Force -ErrorAction SilentlyContinue

# Build the release binary
cargo build --release --bin stage_manager
```

### Build the Installer

The installer (`StageManagerSetup.exe`) embeds the main binary. You must build `stage_manager` first, then point the installer build at it via an environment variable:

```powershell
# Set the path to the release binary
$env:STAGE_MANAGER_EMBED_PATH = "target\release\stage_manager.exe"

# Build the installer
cargo build --release --bin StageManagerSetup
```

### Collect Distribution Outputs

```powershell
# Create dist\ directory
New-Item -ItemType Directory -Force -Path dist

# Copy both binaries
Copy-Item target\release\stage_manager.exe dist\
Copy-Item target\release\StageManagerSetup.exe dist\
```

Your `dist\` folder now contains:
- `stage_manager.exe` — the portable tray app (run directly)
- `StageManagerSetup.exe` — the Windows installer

> [!WARNING]
> PowerShell `.ps1` scripts may be blocked by your system's execution policy. All build steps above are given as individual commands to avoid this. If you see an error about scripts being disabled, run the commands one at a time as shown.

---

## Module Tour

Stage Manager's source is organized into **13 modules** in `src/`, plus a standalone `setup.rs` for the installer binary.

| Module | File | Description |
|--------|------|-------------|
| `animation` | `src/animation.rs` | Spring physics engine (`Spring`, `SpringRect`), `FrameClock`, `VsyncTicker`, DWM flush-based VSync. Drives all smooth transitions. |
| `app` | `src/app.rs` | Top-level multi-monitor coordinator. Owns the Win32 message loop, a `HashMap<(DesktopId, monitor_key), StageManager>` of workspaces, per-monitor `Sidebar` instances. Handles `WM_STAGE_DISPLAY_CHANGE` (monitor hotplug), `WM_STAGE_TRAY_QUEUED` (tray icon fix), `CreateMutexW` single-instance guard, and `MENU_RUN_AT_STARTUP`. |
| `blur` | `src/blur.rs` | Windows 11 Acrylic/Mica glass effects via `DwmSetWindowAttribute`. Exposes the `GlassTheme` enum. |
| `config` | `src/config.rs` | `Config` struct with all settings and their defaults. `SidebarEdge`, `TriggerMode`, `GroupingMode` enums. Single source of truth for all configurable behavior. |
| `desktop` | `src/desktop.rs` | Monitor enumeration (`enumerate_monitors`, `monitor_for_window`, `monitor_for_point`). `VirtualDesktopTracker` wrapping `IVirtualDesktopManager` for per-desktop isolation. `is_run_at_startup_enabled` and `set_run_at_startup` registry helpers. |
| `hotkeys` | `src/hotkeys.rs` | Registers global hotkeys via `RegisterHotKey`: **Win+`** (toggle sidebar), **Win+Alt+Left/Right** (cycle stages). |
| `layout` | `src/layout.rs` | Stage strip geometry: `layout_stage_cascade`, `expand_t` fan-out animation, `StageStripMetrics`, `hit_test_stage_or_window`, `lerp_i32`. Manages `AnimatedStageBadge` with `expand_t` and `expanded_previews`. |
| `renderer` | `src/renderer.rs` | Direct2D squircle icon overlay rendered via `UpdateLayeredWindow`. `draw_icon_tile` draws per-window icons. Cross-fades between a stacked icon cluster and individual squircle icons as a stage expands. |
| `sidebar` | `src/sidebar.rs` | Per-monitor floating strip window. Owns `expand_spring` in `StageAnimState`, `is_cursor_inside` for auto-hide, `on_pointer_click` returning `Option<(StageId, Option<isize>)>`, `new_for_monitor`, and `MonitorInfo`. |
| `stage_manager` | `src/stage_manager.rs` | Core window management logic. Enumerates desktop windows, groups them into `Stage` objects, handles focus switching and window moves across virtual desktops. |
| `tray` | `src/tray.rs` | System tray icon (`Shell_NotifyIconW`), context menu construction, menu command dispatch. Routes tray commands back to `App` to mutate `Config`. |
| `thumbnail` | `src/thumbnail.rs` | DWM Thumbnail API (`DwmRegisterThumbnail`, `DwmUpdateThumbnailProperties`) for live window previews in the sidebar. |
| `window_filter` | `src/window_filter.rs` | Decides which top-level windows are eligible for Stage Manager (skips taskbars, invisible windows, UWP shell windows, etc.). |
| `setup` *(installer)* | `src/bin/setup.rs` | Standalone installer binary (`StageManagerSetup.exe`). Installs to `%LOCALAPPDATA%\StageManager`, creates a Start Menu shortcut via `IShellLinkW`, registers with Windows Add/Remove Programs. Supports `--silent` and `--uninstall` flags. |

---

## How to Add a New Config Option

Follow these steps to add a new setting end-to-end:

### 1. Add the field to `Config` in `src/config.rs`

```rust
pub struct Config {
    // ... existing fields ...
    pub my_new_option: bool,  // Description of what it does
}
```

### 2. Update the `Default` implementation

```rust
impl Default for Config {
    fn default() -> Self {
        Config {
            // ... existing defaults ...
            my_new_option: false,
        }
    }
}
```

### 3. Add a tray menu item in `src/tray.rs`

```rust
// In the menu construction function, add a new item:
AppendMenuW(hmenu, MF_STRING | if config.my_new_option { MF_CHECKED } else { MF_UNCHECKED },
    MENU_MY_NEW_OPTION as usize, w!("My New Option"));
```

Define the menu command ID constant at the top of the file:

```rust
const MENU_MY_NEW_OPTION: u32 = 1234; // pick an unused ID
```

### 4. Handle the menu command in `src/app.rs`

In the `WM_COMMAND` handler (or wherever tray commands are dispatched):

```rust
MENU_MY_NEW_OPTION => {
    app.config.my_new_option = !app.config.my_new_option;
    // Apply any immediate side effects here
}
```

### 5. Use the option in the relevant module

Read `config.my_new_option` wherever it needs to affect behavior — typically in `sidebar.rs`, `renderer.rs`, or `stage_manager.rs`.

> [!TIP]
> If the new option affects rendering, trigger a redraw after changing it. If it affects the sidebar layout, invalidate the layout cache.

---

## Code Style

```powershell
# Format all code (required before committing)
cargo fmt

# Lint — warnings are errors in CI
cargo clippy -- -D warnings
```

> [!IMPORTANT]
> Both `cargo fmt` and `cargo clippy -- -D warnings` must pass with zero issues. CI enforces this. Run them before every commit.

---

## Debug Tips

### App won't start / exits silently

- Check **Event Viewer** → Windows Logs → Application for crash details.
- Run from a PowerShell terminal with `cargo run` to see `eprintln!` output directly.
- Confirm DWM is enabled: right-click desktop → Personalize — if themes are available, DWM is on.

### Sidebar doesn't appear

- Move the cursor slowly to the configured screen edge (or corner, depending on `trigger_mode`).
- If `trigger_mode = TopCorner`, the trigger zone is only the top ~100px of the edge.
- Check that no other app has registered the same global hotkey (`Win+\``).

### Multiple instances / "already running"

Stage Manager uses `CreateMutexW` as a single-instance guard. If the mutex is stuck (e.g. after a crash), kill any lingering `stage_manager.exe` processes:

```powershell
Stop-Process -Name stage_manager -Force -ErrorAction SilentlyContinue
```

### Release build fails with "Access Denied" (OS Error 5)

The running `stage_manager.exe` holds a file lock on itself. Kill it first:

```powershell
Stop-Process -Name stage_manager -Force -ErrorAction SilentlyContinue
cargo build --release --bin stage_manager
```

### High CPU usage

The animation loop uses DWM flush-based VSync (`DwmFlush`) to pace frames. If you see high CPU:
- Confirm DWM compositing is active.
- Check that the sidebar is auto-hiding correctly (the animation loop should sleep when the sidebar is fully hidden).
- Run `cargo build --release` — debug builds are significantly slower due to unoptimized physics calculations.

### Virtual desktop isolation not working

`IVirtualDesktopManager` requires Windows 10 1607+. Verify your Windows build with:

```powershell
winver
```

---

## Project Layout

```
stage-manager-windows/
├── src/
│   ├── main.rs              # Entry point, initializes App
│   ├── app.rs               # Multi-monitor coordinator, message loop
│   ├── animation.rs         # Spring physics, VSync
│   ├── blur.rs              # Acrylic/Mica glass themes
│   ├── config.rs            # Config struct and defaults
│   ├── desktop.rs           # Monitor enumeration, virtual desktop tracker
│   ├── hotkeys.rs           # Global hotkey registration
│   ├── layout.rs            # Stage strip geometry and hit testing
│   ├── renderer.rs          # Direct2D icon overlay
│   ├── sidebar.rs           # Per-monitor sidebar window
│   ├── stage_manager.rs     # Window grouping and focus management
│   ├── thumbnail.rs         # DWM thumbnail previews
│   ├── tray.rs              # System tray icon and context menu
│   ├── window_filter.rs     # Window eligibility filter
│   └── bin/
│       └── setup.rs         # StageManagerSetup.exe installer binary
├── docs/
│   ├── ARCHITECTURE.md
│   ├── CONFIGURATION.md
│   ├── DEVELOPMENT.md       # ← you are here
│   ├── INSTALLATION.md
│   └── TROUBLESHOOTING.md
├── Cargo.toml
└── README.md
```
