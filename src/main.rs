//! Stage Manager for Windows
//!
//! Native Rust implementation using:
//! - DWM Thumbnail API (`DwmRegisterThumbnail`) for live GPU-composited previews
//! - Windows 11 Acrylic (`DWMSBT_TRANSIENTWINDOW`) + rounded glass sidebar
//! - Direct2D + DirectWrite for crisp hardware-accelerated cards, icons & badges
//! - Damped spring physics driven by `DwmFlush()` VSync for 60–144Hz+ motion

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(
    clippy::too_many_arguments,
    clippy::chunks_exact_to_as_chunks,
    clippy::missing_transmute_annotations
)]

mod animation;
mod app;
mod blur;
mod config;
mod desktop;
mod hotkeys;
mod layout;
mod renderer;
mod sidebar;
mod stage_manager;
mod thumbnail;
mod transition;
mod tray;
mod window_tracker;

use app::App;
use log::{info, Level, LevelFilter, Metadata, Record};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::OnceLock;
use std::time::Instant;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};

struct SimpleLogger {
    started: Instant,
}

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let ms = self.started.elapsed().as_millis();
            eprintln!(
                "[{ms:>7}ms {:5} {}] {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

static LOGGER: OnceLock<SimpleLogger> = OnceLock::new();

fn init_logger() {
    let logger = LOGGER.get_or_init(|| SimpleLogger {
        started: Instant::now(),
    });
    if log::set_logger(logger).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}

fn main() {
    init_logger();

    // Ensure only a single Stage Manager tray instance runs per Windows user session.
    let mutex_name: Vec<u16> = OsStr::new("Local\\StageManagerSingleInstance")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let instance_mutex = unsafe { CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr())).ok() };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        info!("Stage Manager is already running in the system tray.");
        if let Some(h) = instance_mutex {
            unsafe {
                let _ = CloseHandle(h);
            }
        }
        return;
    }

    // Enable Per-Monitor V2 DPI awareness before creating any windows
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .expect("Failed to initialize COM");
    }

    info!("Starting Stage Manager for Windows...");

    match App::new() {
        Ok(mut app) => app.run(),
        Err(e) => {
            log::error!("Failed to initialize Stage Manager: {e}");
        }
    }

    unsafe {
        CoUninitialize();
        if let Some(h) = instance_mutex {
            let _ = CloseHandle(h);
        }
    }
}
