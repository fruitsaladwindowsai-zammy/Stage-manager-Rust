# Building from Source

This guide covers building Stage Manager from source code on Windows 10 or 11.

---

## 1. Prerequisites

- **Windows 10 version 20H1+** or **Windows 11**
- **Rust Toolchain (`stable`):** Works with either `x86_64-pc-windows-gnu` or `x86_64-pc-windows-msvc`
- **Git for Windows**

```powershell
winget install --id Git.Git -e --source winget; winget install --id Rustlang.Rustup -e --source winget
```

---

## 2. Compiling the Main Executable

```powershell
# Clone repository
git clone https://github.com/fruitsaladwindowsai-zammy/Stage-manager-Rust.git
cd "Stage-manager-Rust"

# Development build
cargo build

# Release build
cargo build --release --bin stage_manager
```

---

## 3. Building the Standalone Setup Installer

```powershell
# 1. Ensure main app is compiled
cargo build --release --bin stage_manager

# 2. Embed into installer
$env:STAGE_MANAGER_EMBED_PATH = "target\release\stage_manager.exe"
cargo build --release --bin StageManagerSetup

# 3. Output located at target\release\StageManagerSetup.exe
```

---

## 4. Running Unit Tests

```powershell
cargo test
```
All 12 unit tests should pass with zero errors.
