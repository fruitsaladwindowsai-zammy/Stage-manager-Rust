# Build binaries if not already present
if (-not (Test-Path "dist\stage_manager.exe") -or -not (Test-Path "dist\StageManagerSetup.exe")) {
    Write-Host "Building release binaries..." -ForegroundColor Cyan
    cargo build --release --bin stage_manager
    $env:STAGE_MANAGER_EMBED_PATH = "target\release\stage_manager.exe"
    cargo build --release --bin StageManagerSetup
    New-Item -ItemType Directory -Force -Path dist | Out-Null
    Copy-Item target\release\stage_manager.exe dist\
    Copy-Item target\release\StageManagerSetup.exe dist\
}

# Download nuget.exe if not present
if (-not (Test-Path "nuget.exe")) {
    Write-Host "Downloading nuget.exe..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri "https://dist.nuget.org/win-x86-commandline/latest/nuget.exe" -OutFile "nuget.exe" -UseBasicParsing
}

# Pack .nupkg
Write-Host "Creating NuGet package..." -ForegroundColor Green
.\nuget.exe pack StageManager.nuspec

Write-Host "Done! Package created: $(Get-ChildItem -Filter '*.nupkg' | Select-Object -Last 1 -ExpandProperty Name)" -ForegroundColor Green
