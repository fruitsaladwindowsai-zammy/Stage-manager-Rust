# Contributing Guide

Thank you for your interest in contributing to **Stage Manager for Windows**!

---

## 1. Development Workflow

1. **Fork & Branch:** Create a feature branch from `master`:
   ```powershell
   git checkout -b feature/my-cool-feature
   ```
2. **Coding Standards:**
   - Run `cargo fmt` before committing.
   - Ensure `cargo clippy --all-targets -- -D warnings` produces zero warnings.
   - Run `cargo test` to verify all 12 tests pass.
3. **Submitting a Pull Request:**
   - Open a PR against `master`.
   - Describe what changed and include screenshots or GIFs for visual changes.

---

## 2. Safety Guidelines

- Stage Manager maintains safe Rust practices across the codebase.
- Any Win32 API calls must be wrapped cleanly inside `unsafe` blocks with clear invariants.
- Ensure GDI handles (`HICON`, `HBITMAP`, `HDC`) and COM objects are cleaned up properly to prevent resource leaks.
