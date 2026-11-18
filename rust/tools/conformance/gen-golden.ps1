# Builds the reference driver, stores its dump as the golden file, and proves the comparison can
# fail: a single mutated bit must be detected. Windows + Visual Studio C++ Build Tools.
$ErrorActionPreference = 'Stop'

# Make sure the rustup shims are reachable even when the calling shell lacks them.
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }

$conformance = $PSScriptRoot
$root   = (Resolve-Path "$conformance\..\..\..").Path
$outDir = Join-Path $conformance 'out'
$golden = Join-Path $root 'rust\crates\oxidized-imgui-conformance\tests\golden'
New-Item -ItemType Directory -Force -Path $outDir, $golden | Out-Null

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) { throw 'vswhere not found: install Visual Studio C++ Build Tools' }
$inst = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $inst) { throw 'no Visual Studio C++ toolchain found' }
$vcvars = Join-Path $inst 'VC\Auxiliary\Build\vcvars64.bat'

$bat = Join-Path $outDir 'build.bat'
@"
@echo off
call "$vcvars" >nul 2>&1
cd /d "$outDir"
cl /nologo /std:c++17 /EHsc /I"$root" "$conformance\driver.cpp" "$root\imgui.cpp" "$root\imgui_draw.cpp" "$root\imgui_tables.cpp" "$root\imgui_widgets.cpp" /Fe:driver.exe
"@ | Set-Content -Path $bat -Encoding ascii

& cmd /c $bat 2>&1 | Select-Object -Last 4
$driver = Join-Path $outDir 'driver.exe'
if (-not (Test-Path $driver)) { throw 'driver build failed' }

$tmp = Join-Path $outDir 'dump.txt'
& cmd /c "`"$driver`" > `"$tmp`"" 
if ($LASTEXITCODE -ne 0) { throw "driver failed: $LASTEXITCODE" }

Copy-Item $tmp (Join-Path $golden 'prim_rect_uv.txt') -Force
Write-Host "golden written: prim_rect_uv.txt ($((Get-Item $tmp).Length) bytes)"

# Rust dump, captured byte-for-byte through cmd redirection.
$actual = Join-Path $outDir 'rust-dump.txt'
Push-Location (Join-Path $root 'rust')
& cmd /c "cargo run -q -p oxidized-imgui-conformance --bin conformance-dump > `"$actual`"" 
Pop-Location
if ($LASTEXITCODE -ne 0) { throw "cargo run failed: $LASTEXITCODE" }

function Compare-Bytes([string]$a, [string]$b) {
    $ba = [IO.File]::ReadAllBytes($a)
    $bb = [IO.File]::ReadAllBytes($b)
    if ($ba.Length -ne $bb.Length) { return "length differs: $($ba.Length) vs $($bb.Length)" }
    for ($i = 0; $i -lt $ba.Length; $i++) {
        if ($ba[$i] -ne $bb[$i]) { return "first difference at byte $i" }
    }
    return $null
}

$diff = Compare-Bytes $tmp $actual
if ($diff) { throw "FAIL: Rust dump does not match the reference ($diff)" }
Write-Host 'PASS: Rust dump matches the reference byte for byte'

# Negative test: one flipped bit must be caught, otherwise the comparison proves nothing.
$mutated = Join-Path $outDir 'mutated.txt'
$text = [IO.File]::ReadAllText($tmp)
$needle = 'pos 41200000'
if (-not $text.Contains($needle)) { throw "FAIL: cannot find '$needle' to mutate" }
$text.Replace($needle, 'pos 41200001') | Set-Content -Path $mutated -NoNewline -Encoding ascii
$diff = Compare-Bytes $mutated $actual
if (-not $diff) { throw 'FAIL: the comparison did not detect a one-bit difference' }
Write-Host "PASS: negative test detects a one-bit mutation ($diff)"
