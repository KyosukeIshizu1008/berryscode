# BerryCode Windows Release Build
# Usage: .\scripts\build-release.ps1 [-Version "0.2.0"]

param(
    [string]$Version = "0.2.0"
)

$ErrorActionPreference = "Stop"

Write-Host "=== BerryCode Release Build v$Version ===" -ForegroundColor Cyan
Write-Host ""

# Build
Write-Host "[1/2] Building release binary..." -ForegroundColor Yellow
cargo build --release --bin berrycode
Write-Host "  Binary: target\release\berrycode.exe"

# Package
Write-Host "[2/2] Creating zip package..." -ForegroundColor Yellow
$dir = "berrycode-$Version-windows-x86_64"

if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
New-Item -ItemType Directory -Path $dir | Out-Null

Copy-Item "target\release\berrycode.exe" "$dir\"
Copy-Item -Recurse "berrycode\assets" "$dir\assets"
Copy-Item "LICENSE" "$dir\"
Copy-Item "README.md" "$dir\"

$zip = "$dir.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path $dir -DestinationPath $zip
Remove-Item -Recurse -Force $dir

Write-Host ""
Write-Host "=== Done ===" -ForegroundColor Green
Write-Host "  Archive: $zip"
Write-Host ""
Write-Host "To install: extract the zip and run berrycode.exe"
