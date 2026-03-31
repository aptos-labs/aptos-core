@echo off
setlocal

set "LINKER_FLAVOR=%APTOS_LINKER%"
if not "%LINKER_FLAVOR%"=="" goto dispatch

where mold >nul 2>&1
if %ERRORLEVEL%==0 (
  set "LINKER_FLAVOR=mold"
) else (
  where lld-link >nul 2>&1
  if %ERRORLEVEL%==0 (
    set "LINKER_FLAVOR=lld"
  ) else (
    set "LINKER_FLAVOR=system"
  )
)

:dispatch
if /I "%LINKER_FLAVOR%"=="mold" goto mold
if /I "%LINKER_FLAVOR%"=="lld" goto lld
if /I "%LINKER_FLAVOR%"=="system" goto system
echo Unsupported APTOS_LINKER='%LINKER_FLAVOR%'. Use one of: mold, lld, system. 1>&2
exit /b 2

:mold
mold %*
exit /b %ERRORLEVEL%

:lld
lld-link %*
exit /b %ERRORLEVEL%

:system
link %*
exit /b %ERRORLEVEL%
