# Builds the reference driver, stores its dumps as the golden files, and proves the comparison can
# fail: a single mutated bit must be detected. Windows + Visual Studio C++ Build Tools.
$ErrorActionPreference = 'Stop'

# Make sure the rustup shims are reachable even when the calling shell lacks them.
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }

$scenarios = @('prim_rect_uv', 'prim_rect', 'rect_filled', 'line_h', 'window_layout', 'io_mouse_events', 'io_keyboard_events', 'font_metrics', 'clip_stack', 'text_draw', 'button', 'widgets_basic', 'slider_basic', 'style_defaults', 'aa_rect', 'aa_round_rect', 'aa_line', 'aa_strokes', 'aa_bevel', 'arc_steps', 'bezier_points', 'aa_polyline', 'aa_border', 'button_draw', 'window_bg', 'window_border', 'window_title', 'cmd_offsets', 'scene_window_button', 'button_states', 'button_border')

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
cl /nologo /std:c++17 /EHsc /DIMGUI_DISABLE_SSE /I"$root" "$conformance\driver.cpp" "$root\imgui.cpp" "$root\imgui_draw.cpp" "$root\imgui_tables.cpp" "$root\imgui_widgets.cpp" /Fe:driver.exe
"@ | Set-Content -Path $bat -Encoding ascii

& cmd /c $bat 2>&1 | Select-Object -Last 2
$driver = Join-Path $outDir 'driver.exe'
if (-not (Test-Path $driver)) { throw 'driver build failed' }

function Compare-Bytes([string]$a, [string]$b) {
    $ba = [IO.File]::ReadAllBytes($a)
    $bb = [IO.File]::ReadAllBytes($b)
    if ($ba.Length -ne $bb.Length) { return "length differs: $($ba.Length) vs $($bb.Length)" }
    for ($i = 0; $i -lt $ba.Length; $i++) {
        if ($ba[$i] -ne $bb[$i]) { return "first difference at byte $i" }
    }
    return $null
}

$fixtures = Join-Path $root 'rust\crates\oxidized-imgui-conformance\tests\fixtures'
New-Item -ItemType Directory -Force -Path $fixtures | Out-Null

# Scenarios whose inputs the reference can only supply from its own process: the glyph table and
# the atlas line-texture table are only stable inside the run that produced the golden.
$fixture_names = @{
    'text_draw'           = 'text_draw.glyphs.txt'
    'button_draw'         = 'button_draw.glyphs.txt'
    'window_title'        = 'window_title.glyphs.txt'
    'scene_window_button' = 'scene_window_button.glyphs.txt'
    'button_states'       = 'button_states.glyphs.txt'
    'button_border'       = 'button_border.glyphs.txt'
    'aa_strokes'          = 'aa_strokes.lineuvs.txt'
}

foreach ($name in $scenarios) {
    $dump = Join-Path $outDir "dump-$name.txt"
    if ($fixture_names.ContainsKey($name)) {
        $fixture = Join-Path $fixtures $fixture_names[$name]
        & cmd /c "`"$driver`" $name `"$fixture`" > `"$dump`""
    } else {
        & cmd /c "`"$driver`" $name > `"$dump`""
    }
    if ($LASTEXITCODE -ne 0) { throw "driver failed for $name : $LASTEXITCODE" }

    $actual = Join-Path $outDir "rust-$name.txt"
    Push-Location (Join-Path $root 'rust')
    & cmd /c "cargo run -q -p oxidized-imgui-conformance --bin conformance-dump -- $name > `"$actual`""
    $code = $LASTEXITCODE
    Pop-Location
    if ($code -ne 0) { throw "cargo run failed for $name : $code" }

    $diff = Compare-Bytes $dump $actual
    if ($diff) { throw "FAIL [$name]: Rust dump does not match the reference ($diff)" }

    Copy-Item $dump (Join-Path $golden "$name.txt") -Force
    Write-Host "PASS [$name]: matches the reference ($((Get-Item $dump).Length) bytes)"
}

# Negative test: one flipped bit must be caught, otherwise the comparison proves nothing.
$base = Join-Path $outDir 'dump-prim_rect.txt'
$text = [IO.File]::ReadAllText($base)
$needle = 'pos 41200000'
if (-not $text.Contains($needle)) { throw "FAIL: cannot find '$needle' to mutate" }
$mutated = Join-Path $outDir 'mutated.txt'
$text.Replace($needle, 'pos 41200001') | Set-Content -Path $mutated -NoNewline -Encoding ascii
$actualPrim = Join-Path $outDir 'rust-prim_rect.txt'
$diff = Compare-Bytes $mutated $actualPrim
if (-not $diff) { throw 'FAIL: the comparison did not detect a one-bit difference' }
Write-Host "PASS [negative]: one-bit mutation detected ($diff)"
