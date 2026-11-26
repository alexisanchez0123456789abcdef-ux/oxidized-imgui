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
| Button states | resting, hovered and held colours across four frames | `button_states` golden |
| Button border | the aligned two-ring frame drawn after the fill when `FrameBorderSize` is set | `button_border` golden |
| Window background | background rect before the window clip, with empty trailing commands dropped at close | `window_bg` golden |
| Window border | aligned two-ring frame, drawn only when the window has a background | `window_border` golden |
| Title bar | band below the window top, title text inset by border plus padding, drawn after the border | `window_title` golden |
| Window icons | collapse arrow (soft-join stroke with AA ends) and close cross (two lines), drawn between the border and the title, which shifts right to make room | `window_icons` golden |
| Window icons, interaction | hover and held highlights, the press that only counts on release inside, the collapse that kicks in next frame (title bar only, arrow turned right) and the close that flips `p_open` | `window_icons_states` golden |
| Window icons, dragging | the arrow drag: nothing under the 6 px threshold, then the window follows the mouse keeping the grab offset, and the release does not collapse | `window_icons_drag` golden |
| Mouse wheel lock | the window the wheel started on keeps scrolling when the mouse wanders onto another one, until it moves past the drag threshold (or the 0.7 s lock ages out) | `window_wheel_lock` golden |
| Window auto-size, changing content | the size follows the content of the previous frame in both directions, so shrinking and growing both show up one frame later | `window_autosize_change` golden |
| Window auto-size | the window takes its content size plus the padding (and the title bar), one frame behind the content, starting at padding-only on its first frame | `window_autosize` golden |
| Mouse wheel | over a window it scrolls by five font sizes (capped at two thirds of the interior), symmetric up and down, and leaves the scroll alone when the wheel is silent | `window_wheel` golden |
| Child region scroll | a region whose content is taller than it shows **its own** scrollbar, drawn in the host's list like its border, and its content is offset by its own scroll; the wheel over the region scrolls the region and its bar can be dragged, because a region is a window of its own for the hover search | `child_region` golden (four lists across four frames) |
| Child region | `BeginChild` gives a region its own layout, padding, border and **own draw list**, submitted right after its host's; the host's list keeps the border and the child's list carries its own clip | `child_region` golden (two lists per frame) |
| Tooltip | a tooltip is a window at the mouse that takes neither the mouse nor the keyboard, and it measures itself on its first frame like a popup | `tooltip` golden |
| Popup row click | a row of a popup reports the press, and choosing one closes the popup and hands the focus back to its host, which is drawn on top that frame | `popup_click` golden |
| Popup | a popup owns a draw list of its own, paints `PopupBg`, measures itself on its first frame and sits above every window for the mouse | `popup` golden |
| Combo | the box shows its preview and its arrow, pressing it opens a popup as wide as the box, laid out under it and falling back to the other three sides | `combo` golden |
| Selectable | a row fills the work rectangle, highlights with the header colours by state, and its clickable box is expanded by half the item spacing | `selectable` golden |
| Tree node | an unframed tree node: the row only reacts where the arrow and text are while the hover highlight spans the work area, children indent by `IndentSpacing`, and the arrow opens on press | `tree_node` golden |
| Collapsing header | a framed full-width header with the collapsing arrow, header colours on hover and press, and the open state remembered per id; a click on the label toggles on release, a click on the arrow toggles on press | `collapsing_header` golden |
| Window resize, border highlight | the grabbed border is stroked thicker in the separator colour, drawn after the frame so it sits on top | `window_resize_edges` golden (full draw list) |
| Window resize, double click | double-clicking a grip fits the window to its content, which is the content plus padding constrained by the minimum | `window_resize_autofit` golden |
| Window resize cursor | the handles ask for the diagonal or axis arrow, and a border only reports after 40 ms of hover so brushing past an edge does not light it up | `window_resize_cursor` golden |
| Window resize, edges | the lower-left grip and the four borders resize too, moving the opposite corner: a left drag moves the window and grows it at once | `window_resize_edges` golden (geometry) |
| Window resize, dragging | dragging the lower-right grip resizes the window: the size is the mouse position minus the offset inside the handle, constrained by `WindowMinSize` | `window_resize_drag` golden |
| Window resize grips | a resizable window shows the lower-right corner grip, the lower-left one appears only while hovered or held, and the three colours follow the mouse; dragging them does not resize yet | `window_resize_grips` golden |
| Window minimum size | a size asked for by API is clamped up to `WindowMinSize`, while an auto-sized window uses the 1 pixel hard minimum and can end up smaller; a squeezed vertical bar also fades out and stops taking clicks | `window_min_size` golden |
| Scrollbar by page | with `ConfigScrollbarScrollByPage` a click outside the grab scrolls one page, and Shift turns any click back into an absolute seek | `window_scrollbar_page` golden |
| Horizontal scrollbar | shown once the content is wider than the window, taking its height off the bottom of the interior, drawn and draggable through the same axis-generic code as the vertical one | `window_hscrollbar` golden |
| Vertical scrollbar, input | the grab can be grabbed and dragged (absolute seek, so the offset it was grabbed with is kept), a click on the track jumps to that point, and the grab paints its hovered and active colours | `window_scrollbar_drag` golden |
| Vertical scrollbar | shown once the content does not fit, taking its width out of the interior, with the grab sized and placed by the visible fraction (truncated to whole pixels) and the clip following the shrunken interior | `window_scrollbar` golden |
| Window scroll | `SetNextWindowScroll` clamped to `ScrollMax` (which lags one frame behind the content), the content origin moving up with it, and the item rects following | `window_scroll` golden |
| Window moving | a click on a window starts a move drag from the title bar or the body (the default is not title-bar-only), the window follows the mouse keeping the grab offset, and it is clamped so a corner stays inside the padded work area | `windows_move` golden |
| Window stack, order | several windows each with their own list, rendered front to back, the top-most one hovered, a click raising the clicked window in the same frame, and the title bar colour following the focus | `windows_order` golden |
| Command offsets | each command carries the index offset of the buffer when it opened | `cmd_offsets` golden |
| Window close | clip starts below the title band; the empty command from the clip push is dropped and the pop adds one only after geometry | `window_bg`, `window_title` goldens |
| Full scene | titled window with a button: background, band, border, title, fill, label and the three commands | `scene_window_button` golden |
| Label, checkbox | text item rect; checkbox box from frame height plus inner spacing, toggling on release | `widgets_basic` golden |
| Slider (float) | grab region (`GrabMinSize`, 2 px padding), click offset, drag ratio, 3-decimal rounding | `slider_basic` golden |
| Style values | default colours (`GetColorU32`), roundings and border sizes | `style_defaults` golden |
| AA fill | rectangle fringe with miter joins; rounded rectangles from the pinned arc table | `aa_rect`, `aa_round_rect` goldens |
| AA lines | segment quad from the normal, thickness split by the fringe | `aa_line` golden |
| AA strokes with the atlas line textures | the whole `_AddPolyline`/`_AddLine`: stroke positions, AA end caps, soft joins with the clipped bevel, closed paths. The atlas line table travels as a fixture (`tests/fixtures/aa_strokes.lineuvs.txt`) | `aa_strokes` golden |
| AA bevel | sharp corners switch from miter to the two-vertex bevel with its overshoot clamp | `aa_bevel` golden |
| AA auto segmentation | segment count from the tessellation error, and the corner step that follows | `arc_steps` golden |
| Bezier points | recursive flatness subdivision with the curve error of 1.12 px | `bezier_points` golden |
| Polyline stroke | open path, miter joins, simple caps, untextured stroke path | `aa_polyline` golden |
| Border stroke | closed path, miter joins, inside stroke position | `aa_border` golden |

The conformance driver is compiled with `/DIMGUI_DISABLE_SSE`: the reference's default `ImRsqrtPrecise` uses the SSE reciprocal-square-root estimate, whose last bits depend on the CPU model, which would make every AA golden machine-specific. Both sides take the exact `1/sqrt` path instead.
| Clip stack | `PushClipRect`/`PopClipRect`, intersection, fullscreen fallback, command splitting | `clip_stack` golden |

Provisional, **not** parity-checked: everything else. In particular circles, the baked corner
textures for rounded widget frames (a second drawing path next to the geometric one), text wrapping
and clipping, and every widget beyond the ones listed above are not implemented yet.

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
