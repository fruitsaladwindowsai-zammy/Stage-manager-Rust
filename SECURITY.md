# Security Policy

## Scope

This security policy covers the following components of Stage Manager for Windows:

- **`stage_manager.exe`** — the system-tray application
- **`StageManagerSetup.exe`** — the self-contained Windows installer / uninstaller

Third-party dependencies (Rust crates, Windows system DLLs) are outside this scope, though reports about dependency vulnerabilities are welcome.

---

## Security Design

Stage Manager is designed with a minimal, user-scoped footprint:

| Property | Detail |
|---|---|
| **Network access** | ❌ None — no HTTP requests, no sockets, no DNS lookups of any kind |
| **Registry writes** | `HKCU\...` only (current user scope) — no `HKLM` writes, no administrator privileges, no UAC prompts |
| **File system writes** | `%LOCALAPPDATA%\StageManager\` only — no writes to system folders (`C:\Windows\`, `C:\Program Files\`, etc.) |
| **Credential storage** | ❌ None |
| **Telemetry / analytics** | ❌ None — no data is collected or transmitted |
| **Background services** | None — no Windows services, no kernel drivers |
| **Elevated privileges** | ❌ Never requested |

Stage Manager interacts only with public Win32 APIs (`DWM`, `Direct2D`, `Shell_NotifyIconW`, `IVirtualDesktopManager`, `RegisterHotKey`) to render its UI and manage window groupings. It reads window metadata (title, HWND, process name) but does not read window *contents*, inject into other processes, or install hooks that persist beyond its own process lifetime.

---

## Supported Versions

| Version | Supported |
|---|---|
| Latest release | ✅ |
| Older releases | ❌ — please update to the latest version |

---

## Reporting a Vulnerability

> [!CAUTION]
> **Do not** open a public GitHub issue for security vulnerabilities — this exposes details to everyone before a fix is available.

### Preferred Method — GitHub Private Vulnerability Reporting

Use GitHub's built-in **Private Vulnerability Reporting** feature:

1. Go to the repository's **Security** tab.
2. Click **"Report a vulnerability"**.
3. Fill in the details (description, reproduction steps, potential impact).

This creates a private advisory visible only to repository maintainers.

### Alternative — GitHub Issue (Low-Severity Only)

For low-severity issues (e.g., information disclosure with no practical exploit path), you may open a GitHub issue titled **`[SECURITY] <brief description>`**. Maintainers will convert it to a private advisory if needed.

### What to Include

Please include as much of the following as possible:

- Description of the vulnerability and affected component(s)
- Step-by-step reproduction instructions
- Potential impact or attack scenario
- Your Windows version and Stage Manager version
- Any suggested mitigations or patches

---

## Response Timeline

| Stage | Target |
|---|---|
| Initial acknowledgement | Within **5 business days** |
| Triage and severity assessment | Within **10 business days** |
| Fix / patch release | Varies by severity; critical issues are prioritised |

We follow responsible disclosure: a CVE or public advisory will be published after a patch is released, crediting the reporter (unless they prefer to remain anonymous).
