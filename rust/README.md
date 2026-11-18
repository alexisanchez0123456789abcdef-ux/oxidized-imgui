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

Provisional, **not** parity-checked: everything else, including `add_rect_filled` beyond the plain
quad (rounding, anti-aliasing, sub-pixel paths).

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
