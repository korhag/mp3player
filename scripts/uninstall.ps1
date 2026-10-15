# ------------------------------------------------------------
# Script: uninstall.ps1
# Purpose: Removes the per-user Windows install, shortcuts, and
#          file associations. Asks before deleting playlists.
#
# Usage (from Command Prompt or PowerShell):
#   scripts\uninstall.cmd
#   scripts\uninstall.cmd -DeleteData
#   powershell -ExecutionPolicy Bypass -File scripts\uninstall.ps1
# ------------------------------------------------------------

[CmdletBinding()]
param(
    [switch]$DeleteData,
    [switch]$NoPrompt
)

$ErrorActionPreference = "Stop"

$AudioExtensions = @(
    ".mp3", ".flac", ".ogg", ".opus", ".wav", ".aac",
    ".m4a", ".mp4", ".alac", ".aiff", ".aif", ".caf"
)

$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\MP3Player"
$Exe = Join-Path $InstallDir "mp3player.exe"
$ProgId = "MP3Player.AudioFile"
$AppName = "MP3Player"

function Test-Interactive {
    return (-not $NoPrompt) -and [Environment]::UserInteractive -and -not [Console]::IsInputRedirected
}

Get-Process -Name "mp3player" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

$dataDir = $null
if (Test-Path $Exe) {
    $printed = & $Exe --print-data-dir 2>$null
    if ($LASTEXITCODE -eq 0 -and $printed) {
        $dataDir = "$printed".Trim()
    }
}
if (-not $dataDir) {
    $dataDir = Join-Path $env:LOCALAPPDATA "MP3Player\MP3Player\data"
}

if ((Test-Path $dataDir) -and -not $DeleteData -and (Test-Interactive)) {
    $answer = Read-Host "Delete playlists and settings in $dataDir ? [y/N]"
    if ($answer -match "^[yY]") {
        $DeleteData = $true
    }
}

foreach ($extension in $AudioExtensions) {
    $openWith = "HKCU:\Software\Classes\$extension\OpenWithProgids"
    if (Test-Path $openWith) {
        Remove-ItemProperty -Path $openWith -Name $ProgId -ErrorAction SilentlyContinue
    }
    $explorer = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$extension\OpenWithProgids"
    if (Test-Path $explorer) {
        Remove-ItemProperty -Path $explorer -Name $ProgId -ErrorAction SilentlyContinue
    }
}

Remove-Item -Path "HKCU:\Software\Classes\$ProgId" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "HKCU:\Software\Classes\Applications\mp3player.exe" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "HKCU:\Software\MP3Player" -Recurse -Force -ErrorAction SilentlyContinue
Remove-ItemProperty -Path "HKCU:\Software\RegisteredApplications" -Name $AppName -ErrorAction SilentlyContinue
Remove-Item -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\MP3Player" -Recurse -Force -ErrorAction SilentlyContinue

$startLink = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\MP3Player.lnk"
$desktopLink = Join-Path ([Environment]::GetFolderPath("Desktop")) "MP3Player.lnk"
Remove-Item -Path $startLink -Force -ErrorAction SilentlyContinue
Remove-Item -Path $desktopLink -Force -ErrorAction SilentlyContinue

if (Test-Path $InstallDir) {
    Remove-Item -Path $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
}

if ($DeleteData -and (Test-Path $dataDir)) {
    Remove-Item -Path $dataDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "Deleted playlists and settings: $dataDir"
} elseif (Test-Path $dataDir) {
    Write-Host "Kept playlists and settings: $dataDir"
}

Write-Host "MP3Player was removed."
