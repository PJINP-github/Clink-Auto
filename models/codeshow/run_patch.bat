@echo off
setlocal
pushd "%~dp0"

set "STATE=dry"

:menu
cls
echo ==============================================
echo   patch.py - click_automation 4 features
echo ==============================================
echo.
if /i "%STATE%"=="dry" goto show_dry
goto show_exec

:show_dry
echo   Current mode : DRY-RUN
echo   Next ENTER   : python patch.py --dry-run
goto show_help

:show_exec
echo   Current mode : EXECUTE
echo   Next ENTER   : python patch.py
goto show_help

:show_help
echo.
echo   ENTER  = run
echo   N      = reset to DRY-RUN
echo   Q      = quit
echo.
echo ==============================================
set "CHOICE="
set /p "CHOICE=^> "

if /i "%CHOICE%"=="Q" goto end
if /i "%CHOICE%"=="QUIT" goto end
if /i "%CHOICE%"=="N" goto reset_dry
goto run_cmd

:reset_dry
set "STATE=dry"
goto menu

:run_cmd
echo.
echo ==============================================
if /i "%STATE%"=="dry" goto run_dry
goto run_exec

:run_dry
echo   Running: python patch.py --dry-run
echo ==============================================
echo.
python patch.py --dry-run
set "RC=%ERRORLEVEL%"
set "STATE=exec"
goto after_run

:run_exec
echo   Running: python patch.py
echo ==============================================
echo.
python patch.py
set "RC=%ERRORLEVEL%"
set "STATE=dry"
goto after_run

:after_run
echo.
if "%RC%"=="0" goto after_ok
echo [ERROR] patch failed. Check the log above.
goto after_report

:after_ok
echo [OK] patch finished.

:after_report
echo.
echo ==============================================
if /i "%STATE%"=="dry" goto after_next_dry
echo   Next mode: EXECUTE
echo ==============================================
echo.
echo Press any key to return to menu...
pause >nul
goto menu

:after_next_dry
echo   Next mode: DRY-RUN
echo ==============================================
echo.
echo Press any key to return to menu...
pause >nul
goto menu

:end
popd
endlocal