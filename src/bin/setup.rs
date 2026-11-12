//! Standalone Windows Installer & Uninstaller for Stage Manager.
//!
//! - Extracts embedded `stage_manager.exe` (or copies sibling `stage_manager.exe`)
//! - Installs into `%LOCALAPPDATA%\StageManager\`
//! - Enables automatic startup with Windows (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`)
//! - Creates a Start Menu shortcut (`Stage Manager.lnk`)
//! - Registers in Windows Installed Apps (`HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\StageManager`)
//! - Launches Stage Manager directly into the system tray

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

include!(concat!(env!("OUT_DIR"), "/embedded_payload.rs"));

use std::ffi::OsStr;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNO,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const UNINSTALL_SUBKEY: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\StageManager";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let silent = args.iter().any(|a| a == "--silent" || a == "/S");
    let uninstall = args.iter().any(|a| a == "--uninstall" || a == "/U");

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    if uninstall {
        run_uninstall(silent);
    } else {
        run_install(silent);
    }

    unsafe {
        CoUninitialize();
    }
}

fn install_dir() -> Result<PathBuf, String> {
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "LOCALAPPDATA environment variable is not set".to_string())?;
    Ok(PathBuf::from(local_app_data).join("StageManager"))
}

fn start_menu_shortcut_path() -> Option<PathBuf> {
    let app_data = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(app_data)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Stage Manager.lnk"),
    )
}

fn run_install(silent: bool) {
    if !silent {
        let prompt = "Install Stage Manager for Windows?\n\n\
            This setup will:\n\
            • Install to %LOCALAPPDATA%\\StageManager\n\
            • Create a Start Menu shortcut\n\
            • Enable Run at Windows Startup (tray app)\n\
            • Register in Windows Installed Apps for easy uninstall\n\
            • Start Stage Manager now in the system tray";
        let choice = show_msg(
            "Stage Manager Setup",
            prompt,
            MB_YESNO.0 | MB_ICONQUESTION.0,
        );
        if choice != IDYES.0 {
            return;
        }
    }

    match perform_install() {
        Ok(exe_path) => {
            if !silent {
                let msg = format!(
                    "Stage Manager is installed and running in your system tray.\n\n\
                    Location: {}\n\
                    Tip: Move your mouse to any screen edge (or press Win+`) to reveal Stage Manager, \
                    and right-click the tray icon to customize settings.",
                    exe_path.display()
                );
                show_msg("Stage Manager Setup", &msg, MB_OK.0 | MB_ICONINFORMATION.0);
            }
        }
        Err(err) => {
            if !silent {
                show_msg(
                    "Stage Manager Setup Error",
                    &format!("Installation failed:\n\n{err}"),
                    MB_OK.0 | MB_ICONERROR.0,
                );
            }
            std::process::exit(1);
        }
    }
}

fn perform_install() -> Result<PathBuf, String> {
    stop_running_app();

    let dir = install_dir()?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed creating {}: {e}", dir.display()))?;

    let target_exe = dir.join("stage_manager.exe");
    if let Some(bytes) = EMBEDDED_STAGE_MANAGER {
        fs::write(&target_exe, bytes)
            .map_err(|e| format!("Failed writing {}: {e}", target_exe.display()))?;
    } else {
        let current = std::env::current_exe().map_err(|e| e.to_string())?;
        let sibling = current
            .parent()
            .ok_or_else(|| "Cannot resolve installer directory".to_string())?
            .join("stage_manager.exe");
        if !sibling.is_file() {
            return Err(format!(
                "Could not find stage_manager.exe next to installer ({}).",
                sibling.display()
            ));
        }
        fs::copy(&sibling, &target_exe)
            .map_err(|e| format!("Failed copying {}: {e}", sibling.display()))?;
    }

    // Copy installer itself as uninstaller so Installed Apps -> Uninstall works cleanly
    let uninstaller_exe = dir.join("uninstall.exe");
    if let Ok(self_exe) = std::env::current_exe() {
        if self_exe != uninstaller_exe {
            let _ = fs::copy(&self_exe, &uninstaller_exe);
        }
    }

    // Register Windows Boot Startup
    set_reg_sz(
        RUN_SUBKEY,
        "StageManager",
        &format!("\"{}\"", target_exe.display()),
    )?;

    // Register Start Menu shortcut
    if let Some(lnk) = start_menu_shortcut_path() {
        let _ = create_shortcut(&lnk, &target_exe, &dir, "Native Windows Stage Manager");
    }

    // Register Add/Remove Programs entry
    let _ = register_uninstall_entry(&dir, &target_exe, &uninstaller_exe);

    // Launch installed Stage Manager silently into the system tray
    let _ = Command::new(&target_exe)
        .current_dir(&dir)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();

    Ok(target_exe)
}

fn run_uninstall(silent: bool) {
    if !silent {
        let choice = show_msg(
            "Uninstall Stage Manager",
            "Remove Stage Manager from your PC and disable Windows boot startup?",
            MB_YESNO.0 | MB_ICONQUESTION.0,
        );
        if choice != IDYES.0 {
            return;
        }
    }

    stop_running_app();
    remove_run_value("StageManager");
    remove_uninstall_entry();

    if let Some(lnk) = start_menu_shortcut_path() {
        let _ = fs::remove_file(lnk);
    }

    if let Ok(dir) = install_dir() {
        let app_exe = dir.join("stage_manager.exe");
        let _ = fs::remove_file(app_exe);
        let _ = fs::remove_dir_all(&dir);
        // If current process is running from inside install_dir (uninstall.exe), schedule delayed folder cleanup
        if dir.exists() {
            let cmd_line = format!(
                "timeout /T 1 /NOBREAK > NUL & rmdir /S /Q \"{}\"",
                dir.display()
            );
            let _ = Command::new("cmd")
                .args(["/C", &cmd_line])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
    }

    if !silent {
        show_msg(
            "Stage Manager Uninstalled",
            "Stage Manager has been removed.",
            MB_OK.0 | MB_ICONINFORMATION.0,
        );
    }
}

fn stop_running_app() {
    let _ = Command::new("taskkill")
        .args(["/F", "/IM", "stage_manager.exe"])
        .creation_flags(CREATE_NO_WINDOW)
        .status();
    std::thread::sleep(std::time::Duration::from_millis(250));
}

fn create_shortcut(
    lnk_path: &Path,
    target_exe: &Path,
    work_dir: &Path,
    description: &str,
) -> Result<(), String> {
    if let Some(parent) = lnk_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    unsafe {
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("CoCreateInstance(ShellLink) failed: {e}"))?;
        let target_w = wide(&target_exe.display().to_string());
        let dir_w = wide(&work_dir.display().to_string());
        let desc_w = wide(description);

        shell_link
            .SetPath(PCWSTR(target_w.as_ptr()))
            .map_err(|e| e.to_string())?;
        shell_link
            .SetWorkingDirectory(PCWSTR(dir_w.as_ptr()))
            .map_err(|e| e.to_string())?;
        shell_link
            .SetDescription(PCWSTR(desc_w.as_ptr()))
            .map_err(|e| e.to_string())?;

        let persist: IPersistFile = shell_link.cast().map_err(|e| e.to_string())?;
        let lnk_w = wide(&lnk_path.display().to_string());
        persist
            .Save(PCWSTR(lnk_w.as_ptr()), true)
            .map_err(|e| format!("Save shortcut failed: {e}"))?;
    }
    Ok(())
}

fn set_reg_sz(subkey: &str, name: &str, value: &str) -> Result<(), String> {
    let sk = wide(subkey);
    let nm = wide(name);
    let val = wide(value);
    unsafe {
        let mut hkey = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(sk.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut hkey,
            None,
        )
        .ok()
        .map_err(|e| format!("RegCreateKeyExW failed: {e}"))?;
        let bytes = std::slice::from_raw_parts(val.as_ptr() as *const u8, val.len() * 2);
        let res = RegSetValueExW(hkey, PCWSTR(nm.as_ptr()), 0, REG_SZ, Some(bytes)).ok();
        let _ = RegCloseKey(hkey);
        res.map_err(|e| format!("RegSetValueExW failed: {e}"))
    }
}

fn set_reg_dword(subkey: &str, name: &str, value: u32) -> Result<(), String> {
    let sk = wide(subkey);
    let nm = wide(name);
    unsafe {
        let mut hkey = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(sk.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut hkey,
            None,
        )
        .ok()
        .map_err(|e| format!("RegCreateKeyExW failed: {e}"))?;
        let bytes = value.to_le_bytes();
        let res = RegSetValueExW(hkey, PCWSTR(nm.as_ptr()), 0, REG_DWORD, Some(&bytes)).ok();
        let _ = RegCloseKey(hkey);
        res.map_err(|e| format!("RegSetValueExW failed: {e}"))
    }
}

fn remove_run_value(name: &str) {
    let sk = wide(RUN_SUBKEY);
    let nm = wide(name);
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(sk.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        )
        .is_ok()
        {
            let _ = RegDeleteValueW(hkey, PCWSTR(nm.as_ptr()));
            let _ = RegCloseKey(hkey);
        }
    }
}

fn register_uninstall_entry(
    dir: &Path,
    target_exe: &Path,
    uninstaller_exe: &Path,
) -> Result<(), String> {
    set_reg_sz(UNINSTALL_SUBKEY, "DisplayName", "Stage Manager for Windows")?;
    set_reg_sz(
        UNINSTALL_SUBKEY,
        "DisplayVersion",
        env!("CARGO_PKG_VERSION"),
    )?;
    set_reg_sz(UNINSTALL_SUBKEY, "Publisher", "Stage Manager Open Source")?;
    set_reg_sz(
        UNINSTALL_SUBKEY,
        "InstallLocation",
        &dir.display().to_string(),
    )?;
    set_reg_sz(
        UNINSTALL_SUBKEY,
        "DisplayIcon",
        &target_exe.display().to_string(),
    )?;
    set_reg_sz(
        UNINSTALL_SUBKEY,
        "UninstallString",
        &format!("\"{}\" --uninstall", uninstaller_exe.display()),
    )?;
    set_reg_dword(UNINSTALL_SUBKEY, "NoModify", 1)?;
    set_reg_dword(UNINSTALL_SUBKEY, "NoRepair", 1)?;
    Ok(())
}

fn remove_uninstall_entry() {
    let sk = wide(UNINSTALL_SUBKEY);
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(sk.as_ptr()));
    }
}

fn show_msg(title: &str, text: &str, flags: u32) -> i32 {
    let t = wide(title);
    let m = wide(text);
    unsafe {
        MessageBoxW(
            HWND::default(),
            PCWSTR(m.as_ptr()),
            PCWSTR(t.as_ptr()),
            windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE(flags),
        )
        .0
    }
}

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
