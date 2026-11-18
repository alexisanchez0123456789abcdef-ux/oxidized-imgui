//! Compares every scenario dump against the golden file produced by the C++ reference.
//!
//! Regenerate the golden files with `rust/tools/conformance/gen-golden.ps1`, which also proves the
//! comparison can fail (a one-bit mutation must be detected).

use oxidized_imgui_conformance::{SCENARIOS, dump_for};

#[test]
fn golden_matches_reference() {
    for name in SCENARIOS {
        let actual = dump_for(name).unwrap_or_else(|| panic!("unknown scenario {name}"));
        let path = format!("{}/tests/golden/{name}.txt", env!("CARGO_MANIFEST_DIR"));
        let expected =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        assert_eq!(
            actual, expected,
            "scenario {name} differs from the reference dump in {path}"
        );
    }
}
