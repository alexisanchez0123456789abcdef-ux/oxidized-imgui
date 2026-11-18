@echo off
rem Builds and runs the reference probe (the compatibility oracle).
rem Needs Visual Studio Build Tools with the C++ x64 workload.
setlocal enabledelayedexpansion

set "ROOT=%~dp0..\..\.."
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"

if not defined VCVARS (
    for /f "usebackq delims=" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "INST=%%i"
    if defined INST set "VCVARS=!INST!\VC\Auxiliary\Build\vcvars64.bat"
)

if not defined VCVARS (
    echo error: no Visual Studio C++ toolchain found. Set VCVARS to vcvars64.bat.
    exit /b 1
)
if not exist "%VCVARS%" (
    echo error: %VCVARS% not found.
    exit /b 1
)

call "%VCVARS%" >nul 2>&1
cd /d "%~dp0"

cl /nologo /std:c++17 /EHsc /I"%ROOT%" probe.cpp "%ROOT%\imgui.cpp" "%ROOT%\imgui_draw.cpp" "%ROOT%\imgui_tables.cpp" "%ROOT%\imgui_widgets.cpp" /Fe:probe.exe
if errorlevel 1 exit /b 1

probe.exe
