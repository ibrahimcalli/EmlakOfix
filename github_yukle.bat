@echo off
REM Emlak Ofis kaynak kodunu GitHub'a yukler. Git for Windows kurulu olmali.
cd /d "%~dp0"
if not exist .git git init -b main
git add -A
echo.
echo --- Yuklenecek dosyalar (.db .zip .env .gizli ayarlar.json GORMEMELISINIZ) ---
git status --short
set /p c=Devam edilsin mi? (e/h): 
if /i not "%c%"=="e" exit /b 1
git commit -m "Emlak Ofis guncelleme"
git remote get-url origin >nul 2>&1 || git remote add origin https://github.com/ibrahimcalli/EmlakOfix.git
git branch -M main
git push -u origin main
pause
