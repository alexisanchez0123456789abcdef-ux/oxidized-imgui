# Oxidized ImGUI

An implementation of ImGui in Rust, compatible with [Dear ImGui](https://github.com/ocornut/imgui).

**Status: pre-alpha — design and scaffolding.** There is no Rust implementation yet. This
repository currently holds the unmodified Dear ImGui C++ tree (forked at `77fe458`), kept as the
reference the Rust port is checked against.

## What this is

A ground-up Rust implementation of the ImGui immediate-mode GUI model: not a binding, and not a
different API. Widgets are called every frame the same way as upstream, and the identifier stack,
layout rules and draw-list output are meant to match Dear ImGui's semantics, so existing ImGui
code can move over without relearning a framework.

Compatibility is layered. Every layer still has to hold the one above it:

| Layer | Promise | Checked by |
| --- | --- | --- |
| Behaviour | Same widgets, same layout and interaction semantics | conformance tests against the C++ tree |
| C ABI | `cimgui`-shaped C entry points (`igCreateContext`, `igBegin`, ...) | a C program linked against the Rust core |
| C++ source | unchanged `#include "imgui.h"` code compiles against a thin C++ header over that ABI | building upstream examples |

Rust cannot provide C++ ABI compatibility (mangled names, `ImGuiIO` layout, `std::` types in the
headers), so "compatible" means a C ABI at the bottom and a header shim on top — the same route
other language ports take today.

## What this is not

- Not a binding or wrapper of the C++ library: no FFI into Dear ImGui at run time.
- Not a redesign in the spirit of `egui`: the goal is parity with ImGui's model, not a cleaner one.
- Not a binary drop-in replacement for an `imgui` build, and not a C++ ABI shim.
- Not affiliated with, or endorsed by, the Dear ImGui project.

## Repository layout

| Path | Contents |
| --- | --- |
| `imgui*.cpp`, `imgui.h`, `backends/`, `examples/`, `docs/` | upstream Dear ImGui C++ reference, kept intact (see [`docs/README.md`](docs/README.md)) |
| `rust/` | Oxidized ImGui crates — planned, see [`docs/ROADMAP.md`](docs/ROADMAP.md) |

Upstream is vendored as-is so the reference can be rebuilt and diffed at any time; changes on the
Rust side never edit it.

## Why a fork

The C++ tree is the oracle: conformance means "does the Rust produce what this code produces". A
fork keeps the reference, the issues and the diff against `ocornut/imgui` in one place, and lets
each milestone merge the latest upstream before it is declared done. Provenance and the sync
policy are in [`FORK.md`](FORK.md).

## License

MIT, as upstream ([`LICENSE.txt`](LICENSE.txt)); the Dear ImGui copyright notice is preserved.
"Dear ImGui", "ImGui" and its logo belong to Omar Cornut: this project is independent and only
refers to them.
