# Oxidized ImGui — roadmap

Status: pre-alpha. No Rust code yet. This document fixes the order of work and the questions that
must be answered before each milestone. The C++ tree in this repository is the reference.

## Milestones

### M0 — reference and toolchain (in progress)

- [x] Fork with the full branch set; upstream commit pinned in [`FORK.md`](../FORK.md).
- [x] Local clone with an `upstream` remote.
- [ ] Rust toolchain and workspace (`rust/`, `cargo check` green).
- [ ] Conformance harness skeleton: a script that builds the C++ reference and dumps draw lists per
      frame.

### M1 — core model in Rust

`Context`, the identifier stack (hashing must match upstream bit for bit), window and stack
bookkeeping, `ImDrawList` (vertices, indices, commands), and a test backend that emits only the
draw list. Exit test: the same widget script produces the same draw list as the C++ build.

### M2 — C ABI and C++ header

`cimgui`-shaped exports for the M1 surface and a C++ `imgui.h` shim over them. Exit test: an
upstream example compiles untouched and runs against the Rust core.

### M3 — widget and layout parity

Buttons, labels, text inputs, sliders and drags, combos, trees, scroll regions, tooltips, popups,
child windows, tables. Exit test: every widget family has recorded C++/Rust draw-list comparisons.

### M4 — docking, fonts, backends

Docking from `upstream/docking`; font atlas; backend glue for GLFW+OpenGL3 and SDL3+wgpu; upstream
examples ported one row at a time.

## Open decisions

| Question | Why it blocks | Notes |
| --- | --- | --- |
| Identifier hashing | IDs decide interaction identity | upstream builds `ImGuiID` with CRC32 over the ID stack; it must be identical |
| Struct layout | draw data has to match | `f32` matches C `float`; padding and alignment must be pinned with explicit layout |
| Font rasteriser | text geometry and the atlas depend on it | upstream vendors `stb_truetype`; a different rasteriser changes glyph metrics and breaks strict draw-list parity |
| C ABI generation | the surface is large | generate from `imgui.h` rather than hand-writing, and freeze it with the cimgui shape |
| Docking scope | large surface area | land after M3, from the `docking` branch |
| Threading | upstream `Context` is single-threaded | keep single-threaded and document it |

## Non-goals for v1

ImPlot / ImNodes / ImGuizmo, a visual designer, remote or networked UI, a replacement for `egui`,
C++ ABI compatibility, binary drop-in replacement of an existing ImGui build.

## Verification

Behaviour is defined by the C++ reference. A conformance case is a script — create context, set
IO, run frames, inject inputs — run against both implementations; the harness compares draw lists
and widget rects, and the case lives next to the code it protects. Screenshots are a courtesy,
never the pass/fail criterion.
