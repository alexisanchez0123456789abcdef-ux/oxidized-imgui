//! Conformance scenarios and the text dump both implementations must produce.
//!
//! The dump is the contract: `tools/conformance/driver.cpp` prints it from the C++ reference and
//! `tests/golden/` stores that output. The Rust side must reproduce it byte for byte.
//!
//! Floats are printed as the hexadecimal bit pattern, never as decimal text: it makes the diff
//! exact and immune to formatting differences between `printf` and `format!`.

use oxidized_imgui::{DrawList, IMGUI_VERSION, IMGUI_VERSION_NUM, Vec2};

/// `ImDrawListSharedData::TexUvWhitePixel` of the reference's default atlas at pixel density 1.
///
/// This is an input, not something this crate computes: the reference reads it from the font
/// atlas. It is pinned here with the value the driver prints, so the geometry comparison is fair.
/// When the font atlas lands (roadmap F4) it will be derived instead.
pub const WHITE_UV: Vec2 = Vec2 {
    x: 0.000_976_562_5,
    y: 0.003_906_25,
};

/// Every scenario this harness knows, by the name the driver prints.
pub const SCENARIOS: &[&str] = &["prim_rect_uv", "prim_rect", "rect_filled", "line_h"];

const COL: u32 = 0xFFFF_0000; // IM_COL32(0, 0, 255, 255)

/// Builds the draw list for a scenario. `None` when the name is unknown.
pub fn scene(name: &str) -> Option<DrawList> {
    let mut list = DrawList::new();
    list.white_uv = WHITE_UV;
    match name {
        // Explicit UVs: no atlas involved at all.
        "prim_rect_uv" => list.prim_rect_uv(
            Vec2 { x: 10.0, y: 20.0 },
            Vec2 { x: 30.0, y: 50.0 },
            Vec2 { x: 0.25, y: 0.5 },
            Vec2 { x: 0.75, y: 1.0 },
            COL,
        ),
        // White-pixel UV, from the pinned atlas value.
        "prim_rect" => list.prim_rect(Vec2 { x: 10.0, y: 20.0 }, Vec2 { x: 30.0, y: 50.0 }, COL),
        // Public API: pixel-aligned, no rounding, no anti-aliasing.
        "rect_filled" => {
            list.add_rect_filled(Vec2 { x: 10.0, y: 20.0 }, Vec2 { x: 30.0, y: 50.0 }, COL)
        }
        // Public API: horizontal line, default inside stroke, one pixel thick.
        "line_h" => list.add_line_h(10.0, 30.0, 20.0, COL, 1.0),
        _ => return None,
    }
    Some(list)
}

/// The text form compared against the reference.
pub fn dump(scenario: &str, list: &DrawList) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str(&format!("scenario {scenario}\n"));
    out.push_str(&format!(
        "white_uv {:08x} {:08x}\n",
        bits(list.white_uv.x),
        bits(list.white_uv.y)
    ));

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
    fn every_scenario_has_a_dump() {
        for name in SCENARIOS {
            let list = scene(name).expect(name);
            let text = dump(name, &list);
            assert!(text.starts_with("header imgui "), "{name}");
            assert!(text.contains("vtx_count 4"), "{name}");
            assert!(text.contains("idx_count 6"), "{name}");
        }
    }

    #[test]
    fn the_white_uv_is_the_pinned_reference_value() {
        let list = scene("prim_rect").expect("scenario");
        assert_eq!(list.white_uv, WHITE_UV);
        // 1/512 and 1/256 as f32, the values the driver prints.
        assert_eq!(WHITE_UV.x.to_bits(), 0x3a80_0000);
        assert_eq!(WHITE_UV.y.to_bits(), 0x3b80_0000);
    }

    #[test]
    fn unknown_scenario_is_none() {
        assert!(scene("nope").is_none());
    }
}
