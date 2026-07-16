@echo off
REM IRIS Project - Windows Build Script
REM Compiles Rust core and prepares Flutter FFI integration.

setlocal enabledelayedexpansion
set "PROJECT_ROOT=%~dp0"
set "RUST_DIR=%PROJECT_ROOT%iris_flutter\rust"

echo === IRIS Build Script (Windows) ===
echo Project root: %PROJECT_ROOT%

REM ── Build Rust core ──
echo.
echo --- Building iris_bridge (Rust) ---
cd /d "%RUST_DIR%"

echo Building for Windows (MSVC)...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Rust build failed!
    exit /b 1
)

echo Built: target\release\iris_bridge.dll

REM ── Copy to Flutter native directory ──
set "FLUTTER_NATIVE=%PROJECT_ROOT%iris_flutter\native"
if not exist "%FLUTTER_NATIVE%" mkdir "%FLUTTER_NATIVE%"
copy /Y "target\release\iris_bridge.dll" "%FLUTTER_NATIVE%\"
echo Copied to: %FLUTTER_NATIVE%\iris_bridge.dll

echo.
echo === Build complete ===
echo.
echo Next steps:
echo   1. cd iris_flutter
echo   2. flutter pub get
echo   3. flutter run -d windows
