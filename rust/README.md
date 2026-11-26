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
| Keyboard, text events | the character queue (`InputQueueCharacters`) keeps the code points of one frame in order, maps an astral one to U+FFFD exactly as the reference does, and is emptied at the end of the frame; a held key reports `DownDuration` and `DownDurationPrev`, releases once, and repeats after `KeyRepeatDelay` at `KeyRepeatRate`; `Key` is the whole `ImGuiKey` keyboard list, not a subset | `io_text_events` golden |
| Text measurement | `CalcTextSize` over a pinned advance table (the atlas is **not** built: see `font.rs`) | `font_metrics` golden |
| Text | glyph quads including UVs, from the pinned glyph table written by the reference run | `text_draw` golden + `tests/fixtures/text_draw.glyphs.txt` |
| Text, CPU fine clip | `ImFont::RenderText`: glyphs outside the clip are dropped, and with a `cpu_fine_clip_rect` the straddling ones have their position **and UVs** trimmed; the effective rect is the current clip narrowed by the caller's, and one call can serve several clips | `text_clip` golden |
| Text field | a single-line `InputText`: the frame is always `FrameBg`, the text is drawn from `frame_bb.Min + FramePadding` clipped glyph by glyph to the frame, a click activates the field and drops the caret on the nearest character boundary (`stb_text_locate_coord`), a click elsewhere releases it, and the caret is a one-pixel primitive drawn while `CursorAnim` allows | `input_text` golden |
| Text field, editing | typing (`InputQueueCharacters`, Tab and control characters filtered), the caret keys (arrows, Home/End) with Shift dragging the selection, Delete and Backspace, the `TextSelectedBg` rectangle drawn before the text, and the keyboard-driven horizontal scroll in quarter-width steps that only `CursorFollow` moves | `input_text_edit` golden |
| Text field, clipboard | Ctrl+A/C/X/V through the platform clipboard (`Io::clipboard`), Escape restoring the value the field held when it took the keyboard, Enter applying the edit, and the live write-back: `ImGuiItemFlags_LiveEditOnInputText` is on by default, so the caller's buffer follows every edit and the widget reports it that frame | `input_text_clipboard` golden |
| Text field, mouse and undo | dragging extends the selection from the anchor, a double click selects the word under the mouse (walk left to the word start, then right with the `Mac` advance), and Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z walk a full port of `stb_textedit`'s undo stack, which resets when the field takes the keyboard | `input_text_mouse_undo` golden |
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
| Menu bar | a window with the menu bar flag gives up the bar's height at the top, the bar paints `MenuBarBg` inside the border and its entries lay out in a row | `menu_bar` golden |
| Menu columns | the four columns of a vertical menu (icon, label, shortcut, reserved mark) accumulate their widths during a frame and lock them from the next, adding the spacing only between columns that exist | `menu_columns::tests::*`, `submenu` golden |
| Submenu | a menu-bar entry opens a child menu window of its own (`File###Menu_00`), placed below the bar and left-aligned with the entry, measured on its first frame like every popup; its rows fill the content width with the label and the right-aligned shortcut in their measured columns, and choosing one closes the menu and raises the host | `submenu` golden |
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
| Window focus by API | `SetNextWindowFocus` raises the next window to the front of the display order and gives it the focus at `Begin`, without a click: the draw-data order and the `TitleBgActive` bar swap on that same frame | `windows_focus` golden |
| Window collapse by API | `SetNextWindowCollapsed` sets the state `Begin` applies in that same frame: a collapsed window draws only its title bar with the collapse arrow, the body background stays out, and the items submitted into it are skipped (`SkipItems`), so a button neither draws itself nor reports a press | `windows_collapse_api` golden (the collapse arrow needs the line rows in the fixture) |
| Window position by API | `SetWindowPos(name, pos)` moves a live window immediately and truncates the coordinates; the position is the one the next `Begin` reads, so it stays on the following frames until something moves it again | `windows_set_pos` golden |
| Window size by API | `SetWindowSize(name, size)` resizes a live window immediately, truncating each positive axis, so the size the next `Begin` reads is the new one | `windows_set_size` golden |
| Command offsets | each command carries the index offset of the buffer when it opened | `cmd_offsets` golden |
| Window close | clip starts below the title band; the empty command from the clip push is dropped and the pop adds one only after geometry | `window_bg`, `window_title` goldens |
| Full scene | titled window with a button: background, band, border, title, fill, label and the three commands | `scene_window_button` golden |
| Label, checkbox | text item rect; checkbox box from frame height plus inner spacing, toggling on release | `widgets_basic` golden |
| Separator | `Separator` spans the window's work width (`x1` at the cursor, `x2` at `WorkRect.Max.x`), is `max(SeparatorSize, 1)` thick, fills `ImGuiCol_Separator` as a plain rect, and is laid out as an item of `(0, thickness)`, so the next item starts `thickness + ItemSpacing.y` below | `separator` golden |
| Separator in a menu bar | inside a horizontal layout `Separator` takes the vertical form: one pixel wide, as tall as the line the bar already laid out, filling `ImGuiCol_Separator`, and it only advances the horizontal layout (`(thickness, 0)`); the bar's line height is the framed one from `AlignTextToFramePadding` (`FontSize + FramePadding.y * 2`) | `menu_bar_separator` golden |
| Separator with label | `SeparatorText` lays out an item as tall as the label plus `SeparatorTextPadding.y` and as wide as the label plus both paddings, spans its rectangle to the work width, and draws a `SeparatorTextBorderSize` line on each side of the label at the centre of the item; the lines are `AddLineH` with `StrokeCenterBiased`, whose whole-pixel path is a `PrimRect` shifted up by the centre-biased offset | `separator_text` golden |
| Disabled scope | `BeginDisabled`/`EndDisabled` multiply `Style::Alpha` by `Style::DisabledAlpha` (0.6), which every packed colour picks up, and make the items inside neither hover, nor become active, nor report a press; nested scopes only count and `EndDisabled` restores the saved alpha | `disabled` golden |
| Bullet | `Bullet` lays out an item as wide as the font and as tall as the line it lands on (clamped to `FontSize + FramePadding.y * 2`), fills an eight-segment disc of `FontSize * 0.2` at `bb.Min + (FramePadding.x + FontSize * 0.5, line_height * 0.5)`, and keeps the next item on that line with `SameLine(0, FramePadding.x * 2)` | `bullet` golden |
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
| `AddRect` without rounding | the pixel-aligned fast path (whole coordinates, whole thickness, whole pixel density): four quads sharing their corners, white texture, 8 vertices and 24 indices; a fractional rectangle falls back to the closed inside-stroked polyline, and one thinner than its stroke collapses into a filled one | `rect_stroke` golden |
| Circles | `AddCircleFilled`/`AddCircle`: the cached `CircleSegmentCounts` table picks the segment count, `_PathArcToFastEx` samples the pinned 48-entry arc table below `ArcFastRadiusCutoff` (140.1176), the fill is the miter-only `AddConvexPolyFilled`, a radius under `FringeScale·1.414/2` is drawn at that size with its alpha scaled, and a stroke thicker than the radius collapses into a disc | `aa_circles` golden |
| Clip stack | `PushClipRect`/`PopClipRect`, intersection, fullscreen fallback, command splitting | `clip_stack` golden |
| Command split | `_OnChangedClipRect` in full: a clip change closes the command **only when the rectangle really changes**, an unchanged push/pop keeps the geometry in the same command, and a push/pop pair that draws nothing merges its empty command back into the one it came from | `cmd_split` golden (3 commands) |
| Tables, first cut | `BeginTable`/`TableSetupColumn`/`TableNextRow`/`TableNextColumn`/`EndTable` with fixed columns: the column chain from `ImGuiStyle::CellPadding` (which is **(4, 2)**, not `FramePadding`), rows that grow to their content plus one cell padding, the cell cursor at the column's work edge, and the draw splitter's merge that regroups a table's indices by column while its vertices stay in drawing order | `tables_basic` golden |
| Table headers | `TableHeadersRow`: one row as tall as the tallest column name plus two cell paddings, its `TableHeaderBg` band painted into the splitter's **background channel** (which merges before every column channel) across the table's work width, the label at the cell's work edge with the whole cell as its item, and the header row counted by `TableGetRowIndex` | `tables_headers` golden |
| Table row bands | `ImGuiTableFlags_RowBg`: `TableRowBgAlt` (white at 6 % alpha) on every other **data** row, with `TableRowBg` fully transparent and the odd/even counter skipping the header row | `tables_rowbg` golden |
| Table cell backgrounds | `TableSetBgColor`: the fill of a cell is collected while the cell is submitted (the last entry is reused when the column did not change) and painted when the row closes, clipped to the table's background rect and then to the column | `tables_cellbg` golden |
| Table row height | `TableNextRow` with a minimum height: the row is as tall as the tallest of its contents plus one cell padding and the requested minimum, so a `TableNextRow(0, 20)` over 13 px of text gives 20 px rows and pushes the ones below down | `tables_row_height` golden |
| Table inner borders | `ImGuiTableFlags_BordersInner`: the line above every row after the first (`TableBorderStrong` under a header row, `TableBorderLight` otherwise) drawn when the row closes, and one `TableBorderLight` line at the right edge of every column but the last drawn at `EndTable`; the content offsets of bordered cells come from `CellPaddingX` plus the 1 px `CellSpacingX1` | `tables_borders` golden |
| Table outer border | `ImGuiTableFlags_BordersOuter`: the 1 px `TableBorderStrong` rectangle around the table, drawn at `EndTable` through `add_rect_stroke` (the pixel-aligned `AddRect` fast path), with the inner vertical lines shortened by one pixel at each end | `tables_borders_outer` golden |
| Table stretch columns | `ImGuiTableFlags_SizingStretchSame`: a column declared without a width flag takes the policy, the stretch columns share `width_avail - spacings - fixed requests` by weight, the width is truncated with the reference's `+0.01`, and the rounding remainder goes to the right-most stretch column | `tables_stretch` golden |
| Table weighted stretch | `SizingStretchProp` with explicit weights: one fixed column keeps its width, then the two stretch columns divide what is left 2:1 (138 and 69, the latter taking the one-pixel remainder) | `tables_stretch_weights` golden |
| Table explicit column addressing | `TableSetColumnIndex` moves between cells of one row in any order; the row grows to the tallest cell whatever order they are submitted in | `tables_set_column` golden |
| Table column API | `TableGetColumnCount` reports the declared count, `TableGetColumnName` the label a column was declared with (empty before the declaration, null once the table is closed) and `TableGetColumnIndex` the column being built | `tables_names` golden |
| Table header `##id` | a column label may carry a `##id` suffix: `TableGetColumnName` keeps the whole string while the header draws only what `FindRenderedTextEnd` leaves, so `Alpha##1` renders `Alpha` | `tables_header_id` golden |
| Table cell clipping and per-column commands | the items of a cell are clipped to the column (`window.clip_rect` feeds the text's base clip); at `EndTable` the columns whose content stayed inside their clip share one window-clipped command and an overflowing column gets its own command with the column's clip, with the trailing empty command the merge leaves | `tables_clip` golden |
| Table automatic column width | in a `SizingFixedFit` table that is not resizable, a column declared with no width tracks its contents: the first frame gives it `MinColumnWidth` and the width the contents reached is what sizes it from the second frame (kept across frames, as the reference keeps it in its reused table) | `tables_auto_width` golden |
| Table header outside a header row | `TableHeader` submitted in a plain row still fills its cell with `TableHeaderBg`, which is the reference's `else` branch; the item rectangle is the whole cell | `tables_header_inrow` golden |
| Table column flags | `TableGetColumnFlags` reports the flags the table derived at declaration: the width kind from the sizing policy, `NoResize` when the table is not resizable, `IndentEnable` for the first column against `IndentDisable` for the rest, and the `IsEnabled`/`IsVisible` status bits once the layout ran | `tables_column_flags` golden |
| Table row bands after a header row | the header row does not advance the `RowBg` counter, so the first data row keeps `TableRowBg` and the second one takes `TableRowBgAlt` | `tables_rowbg_headers` golden |
| Table `TableNextColumn` walking | the cell advances inside the row until the last column; the call after it starts a new row and opens its first cell, so five calls on a two-column table fill two rows and open a third | `tables_next_column` golden |
| Table header hover | a header is an `AllowOverlap` item, so it only reports hover when it already was the hovered id on the previous frame: with the mouse resting on the cell the `HeaderHovered` fill (`0xCCFA9642`) appears on the third frame, over the row's `TableHeaderBg` band | `tables_header_hover` golden |
| Table header press | pressing a hovered header takes the active id until the button is released, so the cell is filled with `HeaderActive` (`0xFFFA9642`) for as long as it stays down | `tables_header_active` golden |
| Table hovered column and row | `TableGetHoveredColumn` reports the column whose clip holds the mouse (the column count past the last one) on the same frame, while `TableGetHoveredRow` is one frame behind; both use the height the table had on the previous frame for the hit test | `tables_hovered` golden |
| Table proportional stretch | `SizingStretchProp` weights each stretch column by the width its contents reached, so labels `A`, `BB` and `CCC` share the table 1:2:3 (44, 89 and 135 px); with no measurement yet the sum is zero and the NaN weight leaves every column at `MinColumnWidth` on the first frame | `tables_stretch_prop` golden |
| Table explicit row background | `TableSetBgColor(RowBg0, colour)` paints one row with that colour without the `RowBg` flag, and the next row returns to having no background | `tables_row_bg_color` golden |
| Table `TableNextColumn` without a row | calling it before any row opens row 0 and its first cell, so a table can be filled without ever calling `TableNextRow` | `tables_column_bounds` golden |
| Table cell background repetition | a second `TableSetBgColor(CellBg)` on the same cell replaces the first instead of stacking, while the next cell keeps its own colour | `tables_cell_bg_twice` golden |
| Table hovered column flag | `TableGetColumnFlags` carries `IsHovered` (`1 << 27`) on the column whose clip holds the mouse, set on the same frame `TableGetHoveredColumn` reports it and cleared on the rest | `tables_column_hovered_flag` golden |
| Table sort specs | a `Sortable` table exposes one spec per column declared with `DefaultSort` (ascending, in column order), `SpecsDirty` is set when the specs are built and cleared when they are handed out; a table that is not sortable returns none | `tables_sort_specs` golden |
| Table sort specs with `SortTristate` | with no `DefaultSort` the first sortable column becomes the initial spec, so the specs are never empty when they are first handed out; `SortTristate` only allows them to empty later | `tables_sort_tristate` golden |
| Table sorted column flag | `TableGetColumnFlags` carries `IsSorted` (`1 << 26`) on every column a sort spec points at, and the `DefaultSort` bit the column was declared with stays in its flags | `tables_column_sorted_flag` golden |
| Table sort arrow | a sortable column reserves `w_arrow = trunc(FontSize·0.65 + FramePadding.x)` at the right end of its cell and draws there the anti-aliased stroke `RenderArrow` makes — up for ascending, down for descending — before the label; a spec that is not the first one also draws its order number at 70 % alpha (`0xB3FFFFFF`, the reference's float rounding) and moves the arrow left by `ItemInnerSpacing.x + CalcTextSize(number).x`; a `NoSort` column draws nothing. The arrows are strokes that sample the atlas line table, which travels in the fixture | `tables_sort_arrow` golden |
| Table sort click | releasing the mouse over a header it was pressed on applies `TableGetColumnNextSortDirection`: the column cycles `[ascending, descending]` (`PreferSortDescending` swaps them, `SortTristate` appends "none"), an unsorted column starts at the first direction, and without Shift (or without `SortMulti`) every other column loses its order while this one takes order 0. The arrow is drawn before the click is handled, so the frame of the click still shows the old direction; `SpecsDirty` — the flag the caller reads — stays set until the caller clears it | `tables_sort_click` golden |
| Table multi-column sort | with `SortMulti`, a Shift+click appends the clicked column to the specs at `max(SortOrder) + 1` instead of replacing them, so a second header draws its order number and its own arrow; `SpecsDirty` and the single-spec rule stay as above | `tables_sort_multi` golden |
| Table inner scrolling window | `ScrollX`/`ScrollY` makes `BeginTableEx` build the rows in an inner child window with zero padding and border: its own draw list (the frame's draw data carries the child's list instead of the host's empty one), its rectangle clipped to the host's clip, and the row backgrounds drawn with `Bg0ClipRectForDrawCmd` — the **host** window's clip — so the background and the columns split into two commands when they differ. The content fits, so no scrollbar yet | `tables_scroll_child` golden (two lists, 36 vertices) |
| Table column reorder | with `Reorderable`, a held header dragged past its edge queues the display position of the neighbour it crossed; the next `BeginTable` applies the move (`TableSetColumnDisplayOrder` shifting the columns in between, clamped by `NoReorder`), the layout chain follows the display order, and the header keeps `ReorderColumn` while the mouse holds it, which also suppresses the click that would sort it. The draw channels stay in **index** order (that is how the reference allocates and merges them), while the borders follow the display order | `tables_reorder` golden (two moves: B before A, then C before A) |
| Table API after `EndTable` | every getter answers without a table: `TableGetRowIndex` and `TableGetColumnIndex` report 0, the count 0, the hovered column and row -1, the column flags 0, and there are no sort specs | `tables_after_end` golden |

The conformance driver is compiled with `/DIMGUI_DISABLE_SSE`: the reference's default `ImRsqrtPrecise` uses the SSE reciprocal-square-root estimate, whose last bits depend on the CPU model, which would make every AA golden machine-specific. Both sides take the exact `1/sqrt` path instead.

Provisional, **not** parity-checked: everything else. In particular a circle whose radius is past
`ArcFastRadiusCutoff`, or one asked for an explicit segment count, goes through `cos`/`sin` (the
reference's `_PathArcToN`), which no golden pins yet; the baked corner textures for rounded widget
frames (a second drawing path next to the geometric one), text wrapping and clipping, and every
widget beyond the ones listed above are not implemented yet.

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
