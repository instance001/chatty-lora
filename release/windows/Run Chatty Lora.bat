@echo off
setlocal

cd /d "%~dp0"
set "CHATTY_LORA_BASE_PATH=%~dp0"

if not exist "chatty-lora.exe" (
    echo Missing chatty-lora.exe in %~dp0
    pause
    exit /b 1
)

start "" "chatty-lora.exe"
