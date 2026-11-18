# Rust side of Oxidized ImGui

Pre-alpha. This directory is where the implementation lives; everything outside it is upstream
Dear ImGui, kept unmodified as the reference.

## Verified so far

Nothing here is "done"; these are the pieces that already have a test proving them against the
C++ reference instead of against our own expectations:

| Area | Check | Test |
| --- | --- | --- |
| `ImHashStr` / `ImHashData` | CRC32c, seeding, `###` reset: same 32-bit values as `imgui.cpp` | `id::tests::hashes_match_oracle` |
| `ImDrawVert` | 20 bytes, `pos`/`uv`/`col` at 0/8/16 | `draw_list::tests::layouts_match_reference` |
| `ImDrawCmd` | 72 bytes, `ClipRect`/`TexRef`/`VtxOffset`/`IdxOffset`/`ElemCount`/`UserCallback` at 0/16/32/36/40/48 | same |
| ID stack | `get_id`, `push_id`, `pop_id` seed like the reference | `context::tests::ids_nest_like_the_reference` |

Provisional, **not** parity-checked: `DrawList::add_rect_filled` geometry. Producing the reference's
vertices and indices for the same calls is the M1 exit test.

## Commands

```sh
cargo test          # the contract above
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## The oracle

`tools/oracle/build.bat` compiles the upstream `imgui.cpp` and friends with MSVC and runs a probe
that prints version, struct sizes/offsets and hash values. Its output is what the Rust tests
assert. When you add a case, add it to the probe first and let the reference decide the expected
value:

```sh
tools\oracle\build.bat
```

Expected first lines (x86_64 MSVC, 1.93.1):

```
version 1.93.1 WIP 19303
size ImVec2 8
size ImDrawVert 20
size ImDrawCmd 72
```

The probe is a first step of the conformance harness, not the whole thing: it covers constants and
layout. Comparing draw lists from the same widget script needs a scriptable driver on both sides
and comes with M1.
