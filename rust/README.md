# Rust side of Oxidized ImGui

Pre-alpha. This directory is where the implementation lives; everything outside it is upstream
Dear ImGui, kept unmodified as the reference.

## Verified against the reference

Nothing here is "done"; these are the pieces that have a test proving them against the C++ tree
instead of against our own expectations:

| Area | Check | Where |
| --- | --- | --- |
| `ImHashStr` / `ImHashData` | CRC32c, seeding, `###` reset: same 32-bit values as `imgui.cpp` | `id::tests::hashes_match_oracle` |
| `ImDrawVert` / `ImDrawCmd` / `TexRef` | same size and field offsets as the C++ compiler | `draw_list::tests::layouts_match_reference` |
| Identifier stack | `get_id`, `push_id`, `pop_id` seed like the reference | `context::tests::ids_nest_like_the_reference` |
| `PrimRectUV` | identical draw-list dump, byte for byte | `oxidized-imgui-conformance/tests/golden.rs` |
| `PrimRect`, `AddRectFilled` (pixel-aligned), `AddLineH` | identical draw-list dump, byte for byte | same, scenarios `prim_rect`, `rect_filled`, `line_h` |
| Window geometry and cursor layout | style defaults, content region, `Dummy`/`SameLine` advance, remembered pos/size | `window_layout` golden + `context::tests` |
| Mouse input | rounded position, delta, click/release from the down duration, double-click counter | `io_mouse_events` golden |
| Keyboard input | queued events with duplicate filtering, down durations, `IsKeyPressed`, modifiers from `ImGuiMod_*` | `io_keyboard_events` golden |
| Text measurement | `CalcTextSize` over a pinned advance table (the atlas is **not** built: see `font.rs`) | `font_metrics` golden |
| Text | glyph quads including UVs, from the pinned glyph table written by the reference run | `text_draw` golden + `tests/fixtures/text_draw.glyphs.txt` |
| Button | item rect from text metrics, hover, active, release-inside result | `button` golden |
| Button drawing | state colour fill plus the label with its atlas UVs | `button_draw` golden |
| Window background | background rect before the window clip, with empty trailing commands dropped at close | `window_bg` golden |
| Label, checkbox | text item rect; checkbox box from frame height plus inner spacing, toggling on release | `widgets_basic` golden |
| Slider (float) | grab region (`GrabMinSize`, 2 px padding), click offset, drag ratio, 3-decimal rounding | `slider_basic` golden |
| Style values | default colours (`GetColorU32`), roundings and border sizes | `style_defaults` golden |
| AA fill | rectangle fringe with miter joins; rounded rectangles from the pinned arc table | `aa_rect`, `aa_round_rect` goldens |
| AA lines | segment quad from the normal, thickness split by the fringe | `aa_line` golden |
| AA bevel | sharp corners switch from miter to the two-vertex bevel with its overshoot clamp | `aa_bevel` golden |
| AA auto segmentation | segment count from the tessellation error, and the corner step that follows | `arc_steps` golden |
| Bezier points | recursive flatness subdivision with the curve error of 1.12 px | `bezier_points` golden |
| Polyline stroke | open path, miter joins, simple caps, untextured stroke path | `aa_polyline` golden |
| Border stroke | closed path, miter joins, inside stroke position | `aa_border` golden |

The conformance driver is compiled with `/DIMGUI_DISABLE_SSE`: the reference's default `ImRsqrtPrecise` uses the SSE reciprocal-square-root estimate, whose last bits depend on the CPU model, which would make every AA golden machine-specific. Both sides take the exact `1/sqrt` path instead.
| Clip stack | `PushClipRect`/`PopClipRect`, intersection, fullscreen fallback, command splitting | `clip_stack` golden |

Provisional, **not** parity-checked: everything else. In particular the anti-aliased and rounded
paths, the diagonal `AddLine` (stroke joins), circles and any text are not implemented yet; the
font atlas is a prerequisite, because even a solid fill samples the white-pixel UV of the atlas.

## Commands

```sh
cargo test          # unit tests + golden dump comparison
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

## The oracle (constants and layout)

`tools/oracle/build.bat` compiles `imgui.cpp` and friends with MSVC and runs a probe that prints the
version, struct sizes and offsets, and hash values. Its output is what the Rust unit tests assert.
When you add a case, add it to the probe first and let the reference decide the expected value.

## The conformance harness (geometry)

`tools/conformance/driver.cpp` runs a scenario against the reference and prints a deterministic
dump: floats as raw bit patterns, LF only, one line per vertex and index. The Rust side must
reproduce it exactly.

```powershell
# builds the driver, regenerates the golden from the reference, and proves the
# comparison can fail (a one-bit mutation must be detected)
rust\tools\conformance\gen-golden.ps1

cargo test -p oxidized-imgui-conformance   # compares against the stored golden
```

The golden files are committed with `-text` so no checkout can convert their line endings. CI runs
the Rust comparison only; the C++ driver needs Windows and the Visual Studio C++ toolchain, and is
run locally when a scenario is added or the upstream pin changes.

Adding a scenario means, in this order: add it to `driver.cpp`, add the same calls to
`scenarios` in `oxidized-imgui-conformance`, regenerate the golden, and only then make the Rust
match. If the Rust already produces the golden without changes, the scenario proves less than it
looks.
