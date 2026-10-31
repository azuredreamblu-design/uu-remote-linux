@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0test_rumble.ps1" %*
pause
