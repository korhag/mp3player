@echo off
REM Runs the uninstaller without changing the system execution policy.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0uninstall.ps1" %*
