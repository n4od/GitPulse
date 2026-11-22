# Build the Windows *portable* artifact: a zip containing the single executable
# and a short readme. No installer, no registry writes, no admin rights.
#
# The Tauri binary embeds the frontend, so one .exe is genuinely the whole app.
# The only external requirement is the WebView2 runtime, which ships with
# Windows 11 and is present on almost all Windows 10 machines.
#
# Usage: ./scripts/package-portable.ps1 -Tag v0.5.0

param(
    [Parameter(Mandatory = $true)]
    [string]$Tag
)

$ErrorActionPreference = "Stop"

$exe = "src-tauri/target/release/gitpulse.exe"
if (-not (Test-Path $exe)) {
    Write-Error "no release binary at $exe - run 'npm run tauri build' first"
}

$stage = "portable/stage"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

Copy-Item $exe "$stage/GitPulse.exe"

@"
GitPulse $Tag - portable build
================================

Run GitPulse.exe. Nothing is installed and no registry keys are written.

This build needs the Microsoft Edge WebView2 runtime, which is present on
Windows 11 and on most Windows 10 machines. If no window appears, install it:
https://developer.microsoft.com/microsoft-edge/webview2/

Your data lives under %APPDATA%\dev.gitpulse.workflowmonitor. To remove
GitPulse completely, delete that folder and this file.

This build is unsigned. Windows SmartScreen may warn on first run; choose
"More info" then "Run anyway".
"@ | Set-Content "$stage/README.txt" -Encoding utf8

$version = $Tag.TrimStart("v")
$zip = "portable/GitPulse-windows-x64-portable.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }

Compress-Archive -Path "$stage/*" -DestinationPath $zip
Remove-Item $stage -Recurse -Force

Write-Output "wrote $zip"
Get-ChildItem portable -File | Select-Object Name, @{n = 'MB'; e = { [math]::Round($_.Length / 1MB, 2) } }
