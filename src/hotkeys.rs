//! Global hotkey registration for stage switching and sidebar toggle.

use log::{debug, info, warn};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_NOREPEAT, MOD_WIN, VK_LEFT, VK_RIGHT,
};

pub const HOTKEY_TOGGLE_SIDEBAR: i32 = 1;
pub const HOTKEY_NEXT_STAGE: i32 = 2;
pub const HOTKEY_PREV_STAGE: i32 = 3;

pub fn register_hotkeys(hwnd: HWND, toggle_vk: u32) {
    unsafe {
        if RegisterHotKey(
            hwnd,
            HOTKEY_TOGGLE_SIDEBAR,
            MOD_WIN | MOD_NOREPEAT,
            toggle_vk,
        )
        .is_ok()
        {
            info!("Registered hotkey: Win+` (toggle sidebar)");
        } else if RegisterHotKey(
            hwnd,
            HOTKEY_TOGGLE_SIDEBAR,
            MOD_ALT | MOD_NOREPEAT,
            toggle_vk,
        )
        .is_ok()
        {
            info!("Registered fallback hotkey: Alt+` (toggle sidebar)");
        } else {
            warn!("Could not register Win+` or Alt+` hotkey (may be in use)");
        }

        // Note: Ctrl+Win+Left/Right are reserved by Windows 11 for Virtual Desktops,
        // so we use Win+Alt+Right / Win+Alt+Left for stage cycling.
        if RegisterHotKey(
            hwnd,
            HOTKEY_NEXT_STAGE,
            MOD_ALT | MOD_WIN | MOD_NOREPEAT,
            VK_RIGHT.0 as u32,
        )
        .is_ok()
        {
            info!("Registered hotkey: Win+Alt+Right (next stage)");
        } else {
            warn!("Could not register Win+Alt+Right hotkey");
        }

        if RegisterHotKey(
            hwnd,
            HOTKEY_PREV_STAGE,
            MOD_ALT | MOD_WIN | MOD_NOREPEAT,
            VK_LEFT.0 as u32,
        )
        .is_ok()
        {
            info!("Registered hotkey: Win+Alt+Left (previous stage)");
        } else {
            warn!("Could not register Win+Alt+Left hotkey");
        }
    }
}

pub fn unregister_hotkeys(hwnd: HWND) {
    unsafe {
        let _ = UnregisterHotKey(hwnd, HOTKEY_TOGGLE_SIDEBAR);
        let _ = UnregisterHotKey(hwnd, HOTKEY_NEXT_STAGE);
        let _ = UnregisterHotKey(hwnd, HOTKEY_PREV_STAGE);
    }
    debug!("Unregistered global hotkeys");
}
