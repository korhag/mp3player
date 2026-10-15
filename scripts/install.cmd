@echo off
REM Runs the installer without changing the system execution policy.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install.ps1" %*
