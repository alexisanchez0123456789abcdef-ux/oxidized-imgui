# Oxidized ImGui — roadmap

Status: pre-alpha. The Rust crate exists and starts from what can be proven against the C++
reference. This document fixes the order of work and the questions that block each milestone.

## Milestones

### M0 — reference, toolchain and oracle (done)

- [x] Fork with the full branch set; upstream commit pinned in [`FORK.md`](../FORK.md).
- [x] Rust toolchain and workspace in `rust/`; `cargo fmt`, `clippy -D warnings` and `cargo test`
      green.
- [x] Oracle: `rust/tools/oracle` builds `imgui.cpp` with MSVC and prints the version, struct
      sizes and offsets, and hash values. The Rust tests assert those numbers.

### M1 — core model in Rust (in progress)

- [x] `ImHashStr` / `ImHashData`: CRC32c, seeding and `###` reset, bit-identical.
- [x] `ImDrawVert`, `ImDrawCmd`, `TexRef`: same size and field offsets as the C++ compiler.
- [x] Identifier stack: `get_id`, `push_id`, `pop_id`.
- [ ] Draw-list geometry parity: the same vertices and indices as the reference for a rect, a line,
      a circle and text.
- [ ] Frame lifecycle, clip stack and window bookkeeping in `Context`.
- [ ] A scriptable driver on both sides plus a diff tool: the conformance harness proper.

Exit test: the same widget script produces identical draw lists from both implementations.

### M2 — widgets and layout parity

Buttons, labels, text inputs, sliders and drags, combos, trees, scroll regions, tooltips, popups,
child windows, tables. Exit test: every widget family has recorded C++/Rust draw-list comparisons.

### M3 — fonts, docking, backends

Font atlas, then docking from `upstream/docking`, then backend glue (GLFW+OpenGL3, wgpu); upstream
examples ported one row at a time.

### M4 — other languages (not scheduled)

A C ABI and bindings for other languages, only if a real need appears. Not a v1 goal and not a
promise: priority is the Rust crate and behavioural parity.

## Open decisions

| Question | Why it blocks | Notes |
| --- | --- | --- |
| Font rasteriser | text geometry and the atlas depend on it | upstream vendors `stb_truetype`; a different rasteriser changes glyph metrics and breaks draw-list parity |
| Text model | offsets, wrapping and input depend on it | upstream indexes UTF-8 bytes; keep bytes at the core and offer `&str` helpers |
| Public API shape | it is the crate's contract | keep ImGui's call shape (`begin`/`end`, `ui.button`) so porting is mechanical; options via builders |
| Docking scope | large surface area | after M2, from the `docking` branch |
| Backend interface | how draw data reaches a GPU | start with a draw-list-only test backend; add real backends in M3 |
| Threading | upstream `Context` is single-threaded | keep single-threaded and document it |

## Non-goals for v1

ImPlot / ImNodes / ImGuizmo, a visual designer, remote or networked UI, a replacement for `egui`,
C++ ABI or binary compatibility, drop-in replacement of an existing ImGui build.

## Verification

Behaviour is defined by the C++ reference, never by our expectations. A conformance case is a
script — create context, set IO, run frames, inject inputs — that runs against both
implementations; the harness compares draw lists and widget rects. Screenshots are a courtesy,
never the pass/fail criterion.
