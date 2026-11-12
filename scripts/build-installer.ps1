$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Push-Location $root

try {
    Stop-Process -Name stage_manager -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 250

    Write-Host "[1/3] Running tests..."
    cargo test --bin stage_manager

    Write-Host "[2/3] Building stage_manager.exe (release)..."
    cargo build --release --bin stage_manager

    $appExe = Join-Path $root "target\release\stage_manager.exe"
    if (!(Test-Path $appExe)) {
        throw "Missing compiled binary: $appExe"
    }

    Write-Host "[3/3] Building self-contained StageManagerSetup.exe..."
    $env:STAGE_MANAGER_EMBED_PATH = $appExe
    cargo build --release --bin StageManagerSetup
    Remove-Item Env:STAGE_MANAGER_EMBED_PATH -ErrorAction SilentlyContinue

    $dist = Join-Path $root "dist"
    New-Item -ItemType Directory -Force -Path $dist | Out-Null
    Copy-Item $appExe (Join-Path $dist "stage_manager.exe") -Force
    Copy-Item (Join-Path $root "target\release\StageManagerSetup.exe") (Join-Path $dist "StageManagerSetup.exe") -Force

    Write-Host "`nDone! Output files in ${dist}:"
    Get-ChildItem $dist | Select-Object Name, Length
}
finally {
    Remove-Item Env:STAGE_MANAGER_EMBED_PATH -ErrorAction SilentlyContinue
    Pop-Location
}
