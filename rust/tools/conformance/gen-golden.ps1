# Builds the reference driver, stores its dumps as the golden files, and proves the comparison can
# fail: a single mutated bit must be detected. Windows + Visual Studio C++ Build Tools.
$ErrorActionPreference = 'Stop'

# Make sure the rustup shims are reachable even when the calling shell lacks them.
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }

$scenarios = @('prim_rect_uv', 'prim_rect', 'rect_filled', 'line_h', 'window_layout', 'io_mouse_events', 'io_keyboard_events', 'io_text_events', 'font_metrics', 'text_clip', 'input_text', 'input_text_edit', 'input_text_clipboard', 'input_text_mouse_undo', 'clip_stack', 'cmd_split', 'text_draw', 'button', 'widgets_basic', 'slider_basic', 'style_defaults', 'aa_rect', 'aa_round_rect', 'aa_line', 'aa_strokes', 'aa_bevel', 'arc_steps', 'bezier_points', 'aa_polyline', 'aa_border', 'rect_stroke', 'tables_borders_outer', 'tables_names', 'tables_column_flags', 'tables_next_column', 'tables_hovered', 'tables_row_bg_color', 'tables_column_bounds', 'tables_cell_bg_twice', 'tables_column_hovered_flag', 'button_draw', 'window_bg', 'window_border', 'window_title', 'window_icons', 'window_icons_states', 'window_icons_drag', 'windows_order', 'windows_move', 'window_scroll', 'window_scrollbar', 'window_scrollbar_drag', 'window_wheel', 'window_autosize', 'window_autosize_change', 'window_wheel_lock', 'window_hscrollbar', 'window_scrollbar_page', 'window_min_size', 'window_resize_grips', 'window_resize_drag', 'window_resize_edges', 'window_resize_cursor', 'window_resize_autofit', 'collapsing_header', 'tree_node', 'child_region', 'selectable', 'popup', 'popup_click', 'combo', 'tooltip', 'menu_bar', 'submenu', 'cmd_offsets', 'scene_window_button', 'button_states', 'button_border')

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
    'aa_circles'          = 'aa_circles.lineuvs.txt'
    'tables_basic'        = 'tables_basic.glyphs.txt'
    'tables_headers'      = 'tables_headers.glyphs.txt'
    'tables_rowbg'        = 'tables_rowbg.glyphs.txt'
    'tables_cellbg'       = 'tables_cellbg.glyphs.txt'
    'tables_borders'      = 'tables_borders.glyphs.txt'
    'tables_row_height'   = 'tables_row_height.glyphs.txt'
    'tables_stretch_prop' = 'tables_stretch_prop.glyphs.txt'
    'tables_header_id'    = 'tables_header_id.glyphs.txt'
    'tables_clip'         = 'tables_clip.glyphs.txt'
    'tables_auto_width'   = 'tables_auto_width.glyphs.txt'
    'tables_header_inrow' = 'tables_header_inrow.glyphs.txt'
    'tables_rowbg_headers' = 'tables_rowbg_headers.glyphs.txt'
    'tables_header_hover'  = 'tables_header_hover.glyphs.txt'
    'tables_header_active' = 'tables_header_active.glyphs.txt'
    'tables_stretch'      = 'tables_stretch.glyphs.txt'
    'tables_stretch_weights' = 'tables_stretch_weights.glyphs.txt'
    'tables_set_column'   = 'tables_set_column.glyphs.txt'
    'window_resize_edges' = 'window_resize_edges.lineuvs.txt'
    'window_resize_cursor' = 'window_resize_cursor.lineuvs.txt'
    'collapsing_header'   = 'collapsing_header.fixture.txt'
    'tree_node'           = 'tree_node.fixture.txt'
    'child_region'        = 'child_region.glyphs.txt'
    'selectable'          = 'selectable.glyphs.txt'
    'popup'               = 'popup.glyphs.txt'
    'popup_click'         = 'popup_click.glyphs.txt'
    'combo'               = 'combo.glyphs.txt'
    'tooltip'             = 'tooltip.glyphs.txt'
    'text_clip'           = 'text_clip.glyphs.txt'
    'input_text'          = 'input_text.glyphs.txt'
    'input_text_edit'     = 'input_text_edit.glyphs.txt'
    'input_text_clipboard' = 'input_text_clipboard.glyphs.txt'
    'input_text_mouse_undo' = 'input_text_mouse_undo.glyphs.txt'
    'menu_bar'            = 'menu_bar.glyphs.txt'
    'submenu'             = 'submenu.glyphs.txt'
    'window_icons'        = 'window_icons.fixture.txt'
    'window_icons_states' = 'window_icons_states.fixture.txt'
    'window_icons_drag'   = 'window_icons_drag.fixture.txt'
    'windows_order'       = 'windows_order.glyphs.txt'
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

