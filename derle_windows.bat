@echo off
REM Windows icin derleme betigi. Once Rust (rustup) ve "Visual Studio Build Tools - C++" kurulu olmali.
cargo build --release
if errorlevel 1 (echo Derleme basarisiz & pause & exit /b 1)
mkdir "%USERPROFILE%\EmlakOfis" 2>nul
copy /Y target\release\emlak-ofis.exe "%USERPROFILE%\EmlakOfis\"
echo Tamam: %USERPROFILE%\EmlakOfis\emlak-ofis.exe
pause
