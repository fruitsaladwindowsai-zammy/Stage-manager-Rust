//! Configuration types and defaults for macOS-style Stage Manager.

use crate::blur::GlassTheme;

/// Which side of the primary monitor work area the Stage Manager appears on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarEdge {
    Left,
    Right,
}

/// How the auto-hide Stage Manager is triggered by the mouse pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    /// Activate when the mouse enters the top corner on the active side (default).
    TopCorner,
    /// Activate when the mouse enters the bottom corner on the active side.
    BottomCorner,
    /// Activate anywhere along the side edge.
    FullEdge,
}

/// How open windows are grouped into stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupingMode {
    /// Group windows from the same executable into a single stage.
    ByApplication,
    /// Each window forms its own stage unless manually merged via Shift+Click.
    SingleWindow,
}

/// Runtime settings for the sidebar, hot-corner trigger, blur, and animations.
#[derive(Debug, Clone)]
pub struct Config {
    pub sidebar_edge: SidebarEdge,
    pub trigger_mode: TriggerMode,
    pub grouping_mode: GroupingMode,
    pub glass_theme: GlassTheme,
    /// Auto-hide sidebar off-screen until triggered at the corner.
    pub auto_hide: bool,
    /// Show the Gaussian-blurred backdrop strip behind floating stage thumbnails.
    pub blur_backdrop: bool,
    /// Also include the currently focused/active stage in the sidebar strip.
    pub include_active_app: bool,
    /// Logical (96-DPI) width of the sidebar strip.
    pub sidebar_width: i32,
    /// Outer gap from screen edge to floating previews (logical px).
    pub edge_margin: i32,
    /// Vertical gap between stage groups (logical px).
    pub stage_gap: i32,
    /// Maximum number of cascaded preview thumbnails per stage.
    pub max_previews_per_stage: usize,
    /// Virtual key for toggling the sidebar with Win+<key> / Alt+<key> (0xC0 = backtick).
    pub toggle_hotkey_vk: u32,
    /// Whether to animate full-screen transitions with the DWM Transition Overlay.
    pub transition_overlay_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sidebar_edge: SidebarEdge::Left,
            trigger_mode: TriggerMode::FullEdge,
            grouping_mode: GroupingMode::ByApplication,
            glass_theme: GlassTheme::DarkFrosted,
            auto_hide: true,
            blur_backdrop: false,
            include_active_app: false,
            sidebar_width: 224,
            edge_margin: 22,
            stage_gap: 18,
            max_previews_per_stage: 5,
            toggle_hotkey_vk: 0xC0,
            transition_overlay_enabled: true,
        }
    }
}
