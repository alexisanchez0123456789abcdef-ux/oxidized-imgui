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

/// The comparison above walks `SCENARIOS`, so a golden whose scenario is not listed is never
/// compared and its drift goes unnoticed -- which is exactly how `rect_stroke` survived. This walks
/// the directory instead and fails when the two lists disagree.
#[test]
fn every_golden_is_listed_in_scenarios() {
    let dir = format!("{}/tests/golden", env!("CARGO_MANIFEST_DIR"));
    let mut seen = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("golden directory") {
        let path = entry.expect("golden entry").path();
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if path.extension().and_then(|extension| extension.to_str()) != Some("txt") {
            continue;
        }
        seen.push(name.to_string());
    }
    seen.sort();
    let mut unlisted = Vec::new();
    for name in &seen {
        if !SCENARIOS.contains(&name.as_str()) {
            unlisted.push(name.clone());
        }
    }
    assert!(
        unlisted.is_empty(),
        "goldens with no SCENARIOS entry, so nothing compares them: {unlisted:?}"
    );
    assert_eq!(
        seen.len(),
        SCENARIOS.len(),
        "the golden directory and SCENARIOS must hold the same scenarios"
    );
}
