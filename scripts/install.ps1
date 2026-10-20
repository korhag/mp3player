# ------------------------------------------------------------
# Script: install.ps1
# Purpose: Builds the release player and installs it for the
#          current Windows user. No administrator rights.
#
# Usage (from Command Prompt or PowerShell):
#   scripts\install.cmd
#   scripts\install.cmd -Desktop
#   powershell -ExecutionPolicy Bypass -File scripts\install.ps1
#
# Re-run the script to upgrade an existing install.
# ------------------------------------------------------------

[CmdletBinding()]
param(
    [switch]$Desktop,
    [switch]$NoPrompt
)

$ErrorActionPreference = "Stop"

# Keep this list in sync with EXTENSIONS in src/config.rs.
$AudioExtensions = @(
    ".mp3", ".flac", ".ogg", ".opus", ".wav", ".aac",
    ".m4a", ".mp4", ".alac", ".aiff", ".aif", ".caf"
)

$Root = Split-Path -Parent $PSScriptRoot
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\Soundkeep"
$ProgId = "Soundkeep.AudioFile"
$AppName = "Soundkeep"

function Test-Interactive {
    return (-not $NoPrompt) -and [Environment]::UserInteractive -and -not [Console]::IsInputRedirected
}

function Ask-Yes([string]$Prompt) {
    if (-not (Test-Interactive)) {
        return $false
    }
    $answer = Read-Host "$Prompt [y/N]"
    return $answer -match "^[yY]"
}

function Refresh-CargoPath {
    $cargoHome = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path (Join-Path $cargoHome "cargo.exe")) {
        $env:Path = "$cargoHome;" + $env:Path
    }
}

Write-Host "Soundkeep install"
Write-Host "Project: $Root"

Refresh-CargoPath
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust (cargo) was not found."
    $installRust = $NoPrompt -or (Ask-Yes "Install Rust with winget now?")
    if (-not $NoPrompt -and -not $installRust) {
        throw "cargo is required. Install Rust from https://rustup.rs and run this script again."
    }
    if ($NoPrompt -and -not (Get-Command winget -ErrorAction SilentlyContinue)) {
        throw "cargo is required and winget is not available."
    }
    winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements
    Refresh-CargoPath
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo is still not on PATH. Open a new terminal and run this script again."
    }
}

$linker = Get-Command link.exe -ErrorAction SilentlyContinue
if (-not $linker) {
    Write-Warning "link.exe was not found. If the build fails, install Visual Studio Build Tools with the 'Desktop development with C++' workload."
}

# Compiling iced needs several GB. If the project drive is tight, build elsewhere.
$projectDrive = (Get-Item $Root).PSDrive.Name
$projectFree = (Get-PSDrive $projectDrive).Free
$targetDir = Join-Path $Root "target"
if ($projectFree -lt 10GB) {
    $roomy = Get-PSDrive -PSProvider FileSystem |
        Where-Object { $_.Free -gt 10GB -and $_.Name -ne $projectDrive } |
        Sort-Object Free -Descending |
        Select-Object -First 1
    if (-not $roomy) {
        throw "Need about 10 GB free to compile. Free some space and run this script again."
    }
    $targetDir = Join-Path "$($roomy.Name):\" "soundkeep-build"
    Write-Host "Low disk space on ${projectDrive}:. Building in $targetDir"
}
New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
$env:CARGO_TARGET_DIR = $targetDir
$tempBuild = Join-Path $targetDir "tmp"
New-Item -ItemType Directory -Force -Path $tempBuild | Out-Null
$env:TEMP = $tempBuild
$env:TMP = $tempBuild

Write-Host "Building release binary..."
Push-Location $Root
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build --release failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}

$builtExe = Join-Path $targetDir "release\soundkeep.exe"
if (-not (Test-Path $builtExe)) {
    throw "Build finished but $builtExe is missing."
}

$version = "0.1.0"
$cargoToml = Get-Content (Join-Path $Root "Cargo.toml") -Raw
if ($cargoToml -match '(?m)^version\s*=\s*"([^"]+)"') {
    $version = $Matches[1]
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$exe = Join-Path $InstallDir "soundkeep.exe"
$icon = Join-Path $InstallDir "soundkeep.ico"

# A running player locks soundkeep.exe, so Copy-Item cannot replace it.
$running = @(Get-Process -Name "soundkeep" -ErrorAction SilentlyContinue)
if ($running.Count -gt 0) {
    Write-Host "Closing Soundkeep so the new version can replace it..."
    $running | Stop-Process -Force -ErrorAction SilentlyContinue
    $running | Wait-Process -Timeout 10 -ErrorAction SilentlyContinue
}

for ($attempt = 1; $attempt -le 15; $attempt++) {
    try {
        Copy-Item $builtExe $exe -Force -ErrorAction Stop
        break
    } catch [System.IO.IOException] {
        if ($attempt -eq 15) {
            throw "Could not replace $exe because it is still in use. Close Soundkeep and run this script again."
        }
        Start-Sleep -Milliseconds 400
    }
}
Copy-Item (Join-Path $Root "assets\icon.ico") $icon -Force
Copy-Item (Join-Path $PSScriptRoot "uninstall.ps1") (Join-Path $InstallDir "uninstall.ps1") -Force

if (-not $Desktop -and (Test-Interactive)) {
    $Desktop = Ask-Yes "Also create a desktop shortcut?"
}

function New-Shortcut([string]$ShortcutPath, [string]$Target) {
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($ShortcutPath)
    $shortcut.TargetPath = $Target
    $shortcut.WorkingDirectory = $InstallDir
    $shortcut.IconLocation = "$exe,0"
    $shortcut.Description = "Minimalist music player"
    $shortcut.Save()
}

$startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
New-Item -ItemType Directory -Force -Path $startMenu | Out-Null
New-Shortcut (Join-Path $startMenu "Soundkeep.lnk") $exe
if ($Desktop) {
    New-Shortcut (Join-Path ([Environment]::GetFolderPath("Desktop")) "Soundkeep.lnk") $exe
}

$command = "`"$exe`" `"%1`""
$progPath = "HKCU:\Software\Classes\$ProgId"
New-Item -Path $progPath -Force | Out-Null
Set-Item -Path $progPath -Value "Audio file"
New-Item -Path "$progPath\DefaultIcon" -Force | Out-Null
Set-Item -Path "$progPath\DefaultIcon" -Value "$icon,0"
New-Item -Path "$progPath\shell\open\command" -Force | Out-Null
Set-Item -Path "$progPath\shell\open\command" -Value $command

$appKey = "HKCU:\Software\Classes\Applications\soundkeep.exe"
New-Item -Path $appKey -Force | Out-Null
New-ItemProperty -Path $appKey -Name "FriendlyAppName" -Value $AppName -PropertyType String -Force | Out-Null
New-Item -Path "$appKey\shell\open\command" -Force | Out-Null
Set-Item -Path "$appKey\shell\open\command" -Value $command
New-Item -Path "$appKey\SupportedTypes" -Force | Out-Null

$capabilities = "HKCU:\Software\Soundkeep\Capabilities"
New-Item -Path $capabilities -Force | Out-Null
New-ItemProperty -Path $capabilities -Name "ApplicationName" -Value $AppName -PropertyType String -Force | Out-Null
New-ItemProperty -Path $capabilities -Name "ApplicationDescription" -Value "Minimalist music player" -PropertyType String -Force | Out-Null
New-Item -Path "$capabilities\FileAssociations" -Force | Out-Null

foreach ($extension in $AudioExtensions) {
    $extKey = "HKCU:\Software\Classes\$extension"
    New-Item -Path "$extKey\OpenWithProgids" -Force | Out-Null
    New-ItemProperty -Path "$extKey\OpenWithProgids" -Name $ProgId -Value "" -PropertyType String -Force | Out-Null

    $explorerKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$extension\OpenWithProgids"
    New-Item -Path $explorerKey -Force | Out-Null
    try {
        New-ItemProperty -Path $explorerKey -Name $ProgId -Value ([byte[]]@()) -PropertyType Binary -Force | Out-Null
    } catch {
        Write-Warning "Could not register Explorer Open With for $extension"
    }

    New-ItemProperty -Path "$appKey\SupportedTypes" -Name $extension -Value "" -PropertyType String -Force | Out-Null
    New-ItemProperty -Path "$capabilities\FileAssociations" -Name $extension -Value $ProgId -PropertyType String -Force | Out-Null
}

$registered = "HKCU:\Software\RegisteredApplications"
New-Item -Path $registered -Force | Out-Null
New-ItemProperty -Path $registered -Name $AppName -Value "Software\Soundkeep\Capabilities" -PropertyType String -Force | Out-Null

$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Soundkeep"
New-Item -Path $uninstallKey -Force | Out-Null
$uninstallCommand = "powershell.exe -ExecutionPolicy Bypass -File `"$InstallDir\uninstall.ps1`""
$sizeKb = [int]((Get-Item $exe).Length / 1KB)
New-ItemProperty -Path $uninstallKey -Name "DisplayName" -Value $AppName -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name "DisplayVersion" -Value $version -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name "Publisher" -Value $AppName -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name "InstallLocation" -Value $InstallDir -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name "UninstallString" -Value $uninstallCommand -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name "DisplayIcon" -Value $exe -PropertyType String -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name "NoModify" -Value 1 -PropertyType DWord -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name "NoRepair" -Value 1 -PropertyType DWord -Force | Out-Null
New-ItemProperty -Path $uninstallKey -Name "EstimatedSize" -Value $sizeKb -PropertyType DWord -Force | Out-Null

if (-not ("SoundkeepShellNotify" -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class SoundkeepShellNotify {
    [DllImport("shell32.dll")]
    public static extern void SHChangeNotify(int eventId, uint flags, IntPtr item1, IntPtr item2);
}
"@
}
# SHCNE_ASSOCCHANGED tells Explorer to reload file associations.
[SoundkeepShellNotify]::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)

Write-Host ""
Write-Host "Installed $AppName $version"
Write-Host "Program: $exe"
Write-Host "Open it from the Start Menu, or right-click an audio file -> Open with -> Soundkeep."
Write-Host "Windows does not let an installer set the default app. Choose 'Always' once in Open with."
Write-Host "To remove it, run scripts\uninstall.ps1"
