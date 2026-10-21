@echo off
rem Build MD Lite (Rust) and copy md-lite.exe next to this script.
cd /d "%~dp0"
if not exist md-lite.ico powershell -NoProfile -ExecutionPolicy Bypass -File make-icon.ps1
cargo build --release && copy /y target\release\md-lite.exe md-lite.exe >nul && echo md-lite.exe OK
