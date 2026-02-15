@echo off
REM Production build script for Linkr (Windows)
REM This script builds an optimized release binary

echo.
echo Building Linkr for production...
echo.

REM Check if in correct directory
if not exist "Cargo.toml" (
    echo Error: Cargo.toml not found. Run this from the project root.
    exit /b 1
)

REM Clean previous builds
echo Cleaning previous builds...
cargo clean

REM Build release binary
echo Building release binary...
cargo build --release

REM Check if binary exists
if exist "target\release\linkr.exe" (
    echo.
    echo Build successful!
    echo Binary location: target\release\linkr.exe
    echo.
    echo To run locally:
    echo   set DATABASE_URL=postgres://user:pass@localhost/linkr
    echo   set PUBLIC_BASE_URL=http://localhost:3000
    echo   .\target\release\linkr.exe
) else (
    echo Build failed - binary not found
    exit /b 1
)
