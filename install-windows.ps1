# LibreDrop Windows Setup Script
# Copies libredrop.exe and creates convenience shortcuts

$ErrorActionPreference = "Stop"
Write-Host "--- LibreDrop Windows Setup ---" -ForegroundColor Cyan

$CurrentDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ExePath = Join-Path $CurrentDir "libredrop.exe"

if (-not (Test-Path $ExePath)) {
    Write-Host "Error: libredrop.exe not found in $CurrentDir" -ForegroundColor Red
    exit 1
}

$InstallDir = "$env:LOCALAPPDATA\Programs\LibreDrop"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

Copy-Item -Path "$CurrentDir\*" -Destination $InstallDir -Recurse -Force
Write-Host "Installed files to: $InstallDir" -ForegroundColor Green

# Add to User PATH if not already present
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to User PATH." -ForegroundColor Green
}

# Create Desktop Shortcut for Web Gateway
$WshShell = New-Object -ComObject WScript.Shell
$DesktopPath = [Environment]::GetFolderPath("Desktop")
$Shortcut = $WshShell.CreateShortcut("$DesktopPath\LibreDrop Web.lnk")
$Shortcut.TargetPath = "$InstallDir\run-libredrop-web.bat"
$Shortcut.WorkingDirectory = $InstallDir
$Shortcut.Description = "Launch LibreDrop Web Gateway"
$Shortcut.Save()

Write-Host "Desktop shortcut created: LibreDrop Web.lnk" -ForegroundColor Green
Write-Host "Setup complete! You can now run 'libredrop' from any Command Prompt or PowerShell." -ForegroundColor Cyan
