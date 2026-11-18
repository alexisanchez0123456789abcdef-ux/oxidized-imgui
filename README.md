# Oxidized ImGUI

An implementation of ImGui in Rust, compatible with [Dear ImGui](https://github.com/ocornut/imgui).

**Status: pre-alpha.** The Rust crate exists under [`brust/`](rust/) and starts from the parts that
can be proven against the reference: identifier hashing and draw-data layout already match the C++
tree bit for bit, checked by tests. Widgets, layout and interaction are not implemented yet.

## What this is

A ground-up Rust implementation of ImGui's immediate-mode model: not a binding, and not a
different API. Widgets are called every frame the same way as upstream, and the identifier stack,
layout rules and draw data are meant to match Dear ImGui's semantics, so existing ImGui knowledge
carries over unchanged.

Compatibility is pursued in this order. Each row assumes the row above holds:

| Goal | Meaning | State |
| --- | --- | --- |
| Behaviour | same geometry, visibility and draw data for the same sequence of calls | the contract; oracle + first tests in place |
| Rust API | a Rust crate shaped like ImGui: same calls, same semantics, safe by default | in progress |
| Other languages | a C ABI and bindings, if a real need appears | later, not promised |

**No C++ binary compatibility is attempted.** Rust cannot reproduce C++ mangled symbols, `ImGuiIO`
layout or `std::` types across the boundary, and a drop-in replacement of an existing ImGui build
is not a goal. What matters here is that a Rust program gets ImGui's behaviour and ImGui's shape.

## What this is not

- Not a binding or wrapper of the C++ library: no FFI into Dear ImGui at run time.
- Not a redesign in the spirit of `egui`: the goal is parity with ImGui's model, not a cleaner one.
- Not a C++ ABI shim, and not affiliated with or endorsed by the Dear ImGui project.

## Repository layout

| Path | Contents |
| --- | --- |
| `imgui*.cpp`, `imgui.h`, `backends/`, `examples/`, `docs/` | upstream Dear ImGui C++ reference, kept intact (see [`docs/README.md`](docs/README.md)) |
| [`rust/`](rust/) | the Rust crate, and `tools/oracle`, which builds the reference and prints the values the tests assert |

Upstream is vendored as-is so the reference can be rebuilt and diffed at any time; the Rust side
never edits it.

## Why a fork
The C++ tree is the oracle: compatibility means "the Rust does what this code does". A fork keeps
the reference, the issues and the diff against `ocornut/imgui` in one place. Provenance, scope and
the sync policy are in [`FORK.md`](FORK.md); the plan is in [`docs/ROADMAP.md`](docs/ROADMAP.md).

## License

MIT, as upstream ([`LICENSE.txt`](LICENSE.txt)); the Dear ImGui copyright notice is preserved.
"Dear ImGui", "ImGui" and its logo belong to Omar Cornut: this project is independent and only
refers to them.
