@echo off
cd /d "%~dp0"

:loop
cls
echo ==============================================
echo   codeLook.py  (Rust project snapshot)
echo   workdir: %~dp0
echo ==============================================
echo.

:: 清理旧的 seg-*.txt（保留 tree.txt，稍后会被覆盖）
set "SEG_DIR=%~dp0segments"
if exist "%SEG_DIR%\seg-*.txt" (
    echo [CLEAN] Found existing seg-*.txt in segments, clearing...
    del /q "%SEG_DIR%\seg-*.txt" >nul 2>nul
    if errorlevel 1 (
        echo [CLEAN] WARN: some files could not be deleted ^(in use?^)
    ) else (
        echo [CLEAN] Done.
    )
) else (
    echo [CLEAN] No existing seg-*.txt, skip.
)
echo.

python codeLook.py %*
set "RC=%ERRORLEVEL%"

echo.
if "%RC%"=="0" (
    echo [OK] Snapshot generated.
    echo Output dir: %SEG_DIR%\seg-*.txt
    echo Read-tree : %SEG_DIR%\tree.txt
) else (
    echo [ERROR] Snapshot failed. Check the log above.
)

echo.
echo ==============================================
echo   Press ENTER to run again, or type Q to quit.
echo ==============================================
set "CHOICE="
set /p "CHOICE=^> "
if /i "%CHOICE%"=="Q" goto :eof
if /i "%CHOICE%"=="QUIT" goto :eof
goto loop