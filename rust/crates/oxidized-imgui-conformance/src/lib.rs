//! Conformance scenarios and the text dump both implementations must produce.
//!
//! The dump is the contract: `tools/conformance/driver.cpp` prints it from the C++ reference and
//! `tests/golden/` stores that output. The Rust side must reproduce it byte for byte.
//!
//! Floats are printed as the hexadecimal bit pattern, never as decimal text: it makes the diff
//! exact and immune to formatting differences between `printf` and `format!`.
//!
//! Scope today: `PrimRectUV` — the lowest-level quad the reference has, with no font atlas, no
//! anti-aliasing and no rounding involved. Everything else is still provisional.

use oxidized_imgui::{DrawList, IMGUI_VERSION, IMGUI_VERSION_NUM, Vec2};

/// Every scenario this harness knows, by the name the driver prints.
pub const SCENARIOS: &[&str] = &["prim_rect_uv"];

/// Builds the draw list for a scenario. `None` when the name is unknown.
pub fn scene(name: &str) -> Option<DrawList> {
    match name {
        "prim_rect_uv" => Some(scene_prim_rect_uv()),
        _ => None,
    }
}

/// A single `PrimRectUV` with non-trivial positions and UVs, so a swapped corner or a wrong index
/// order shows up in the diff.
pub fn scene_prim_rect_uv() -> DrawList {
    let mut list = DrawList::new();
    list.prim_rect_uv(
        Vec2 { x: 10.0, y: 20.0 },
        Vec2 { x: 30.0, y: 50.0 },
        Vec2 { x: 0.25, y: 0.5 },
        Vec2 { x: 0.75, y: 1.0 },
        // IM_COL32(0, 0, 255, 255): r | g << 8 | b << 16 | a << 24
        0xFFFF_0000,
    );
    list
}

/// The text form compared against the reference.
pub fn dump(scenario: &str, list: &DrawList) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str(&format!("scenario {scenario}\n"));

    out.push_str(&format!("cmd_count {}\n", list.cmd_buffer.len()));
    for (i, cmd) in list.cmd_buffer.iter().enumerate() {
        out.push_str(&format!(
            "cmd {i} clip {:08x} {:08x} {:08x} {:08x} elems {} vtxoff {} idxoff {}\n",
            bits(cmd.clip_rect[0]),
            bits(cmd.clip_rect[1]),
            bits(cmd.clip_rect[2]),
            bits(cmd.clip_rect[3]),
            cmd.elem_count,
            cmd.vtx_offset,
            cmd.idx_offset,
        ));
    }

    out.push_str(&format!("vtx_count {}\n", list.vtx_buffer.len()));
    for (i, v) in list.vtx_buffer.iter().enumerate() {
        out.push_str(&format!(
            "vtx {i} pos {:08x} {:08x} uv {:08x} {:08x} col {:08x}\n",
            bits(v.pos.x),
            bits(v.pos.y),
            bits(v.uv.x),
            bits(v.uv.y),
            v.col,
        ));
    }

    out.push_str(&format!("idx_count {}\n", list.idx_buffer.len()));
    for idx in &list.idx_buffer {
        out.push_str(&format!("idx {idx}\n"));
    }
    out
}

fn bits(value: f32) -> u32 {
    value.to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_is_stable_for_the_rect_scene() {
        let list = scene("prim_rect_uv").expect("scenario");
        let text = dump("prim_rect_uv", &list);
        assert!(text.starts_with("header imgui "));
        assert!(text.contains("vtx_count 4"));
        assert!(text.contains("idx_count 6"));
        // 10.0f and 20.0f as f32 bits, and the color written as eight hex digits.
        assert!(text.contains("vtx 0 pos 41200000 41a00000"));
        assert!(text.contains("col ffff0000"));
    }

    #[test]
    fn unknown_scenario_is_none() {
        assert!(scene("nope").is_none());
    }
}
