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
pub const SCENARIOS: &[&str] = &[
    "prim_rect_uv",
    "prim_rect",
    "rect_filled",
    "line_h",
    "window_layout",
    "io_mouse_events",
    "io_keyboard_events",
    "font_metrics",
    "clip_stack",
    "text_draw",
];

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

/// The `window_layout` scenario: style values, a fixed-size window, two items and a same-line
/// item, then the window's remembered position on the next frame.
pub fn window_layout_dump() -> String {
    use oxidized_imgui::Context;

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_layout\n");

    let style = ctx.style();
    out.push_str(&format!(
        "style wp {:08x} {:08x} is {:08x} {:08x} fp {:08x} {:08x}\n",
        bits(style.window_padding.x),
        bits(style.window_padding.y),
        bits(style.item_spacing.x),
        bits(style.item_spacing.y),
        bits(style.frame_padding.x),
        bits(style.frame_padding.y),
    ));

    let pos = Vec2 { x: 60.0, y: 60.0 };
    let size = Vec2 { x: 300.0, y: 200.0 };
    ctx.begin_frame();
    ctx.begin_window("W", Some(pos), Some(size));

    let w = ctx.window();
    out.push_str(&format!(
        "w0 pos {:08x} {:08x}\n",
        bits(w.pos.x),
        bits(w.pos.y)
    ));
    out.push_str(&format!(
        "w0 size {:08x} {:08x}\n",
        bits(w.size.x),
        bits(w.size.y)
    ));
    let content = ctx.content_avail();
    out.push_str(&format!(
        "w0 content {:08x} {:08x}\n",
        bits(content.x),
        bits(content.y)
    ));
    let cursor = ctx.cursor_screen_pos();
    out.push_str(&format!(
        "w0 cursor {:08x} {:08x}\n",
        bits(cursor.x),
        bits(cursor.y)
    ));

    ctx.dummy(Vec2 { x: 100.0, y: 20.0 });
    let min = ctx.item_rect_min();
    let max = ctx.item_rect_max();
    out.push_str(&format!(
        "w0 item0 min {:08x} {:08x}\n",
        bits(min.x),
        bits(min.y)
    ));
    out.push_str(&format!(
        "w0 item0 max {:08x} {:08x}\n",
        bits(max.x),
        bits(max.y)
    ));
    let cursor = ctx.cursor_screen_pos();
    out.push_str(&format!(
        "w0 cursor_after0 {:08x} {:08x}\n",
        bits(cursor.x),
        bits(cursor.y)
    ));

    ctx.same_line();
    let cursor = ctx.cursor_screen_pos();
    out.push_str(&format!(
        "w0 sameline {:08x} {:08x}\n",
        bits(cursor.x),
        bits(cursor.y)
    ));

    ctx.dummy(Vec2 { x: 50.0, y: 10.0 });
    let min = ctx.item_rect_min();
    let max = ctx.item_rect_max();
    out.push_str(&format!(
        "w0 item1 min {:08x} {:08x}\n",
        bits(min.x),
        bits(min.y)
    ));
    out.push_str(&format!(
        "w0 item1 max {:08x} {:08x}\n",
        bits(max.x),
        bits(max.y)
    ));
    let cursor = ctx.cursor_screen_pos();
    out.push_str(&format!(
        "w0 cursor_after1 {:08x} {:08x}\n",
        bits(cursor.x),
        bits(cursor.y)
    ));
    ctx.end_window();

    // Second frame: no explicit size, the window keeps what it had.
    ctx.begin_window("W", None, None);
    let w = ctx.window();
    out.push_str(&format!(
        "w1 pos {:08x} {:08x}\n",
        bits(w.pos.x),
        bits(w.pos.y)
    ));
    out.push_str(&format!(
        "w1 size {:08x} {:08x}\n",
        bits(w.size.x),
        bits(w.size.y)
    ));
    ctx.end_window();
    ctx.end_frame();
    out
}

/// The `io_mouse_events` scenario: five frames of raw mouse input, printing what `begin_frame`
/// derives from it (rounded position, delta, click, release, down duration, click count).
pub fn io_mouse_dump() -> String {
    use oxidized_imgui::Context;

    let frames: [(f32, f32, bool); 5] = [
        (10.5, 20.5, false),
        (12.7, 22.2, true),
        (14.0, 24.0, true),
        (14.0, 24.0, false),
        (14.0, 24.0, true),
    ];

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario io_mouse_events\n");

    for (i, (x, y, down)) in frames.iter().enumerate() {
        {
            let io = ctx.io_mut();
            io.delta_time = 0.1;
            io.mouse_pos = Some(Vec2 { x: *x, y: *y });
            io.mouse_down[0] = *down;
        }
        ctx.begin_frame();

        let io = ctx.io();
        let pos = io.mouse_pos.unwrap_or_default();
        out.push_str(&format!(
            "f{i} pos {:08x} {:08x} delta {:08x} {:08x} clk {} rel {} count {} dur {:08x}\n",
            bits(pos.x),
            bits(pos.y),
            bits(io.mouse_delta.x),
            bits(io.mouse_delta.y),
            io.mouse_clicked[0] as u8,
            io.mouse_released[0] as u8,
            io.mouse_clicked_count[0],
            bits(io.mouse_down_duration[0]),
        ));
        ctx.end_frame();
    }
    out
}

/// The `io_keyboard_events` scenario: queued key events, a duplicate, two different keys in one
/// frame, the derived down durations and the modifier flags.
pub fn io_keyboard_dump() -> String {
    use oxidized_imgui::{Context, Key};

    let frames: [&[(Key, bool)]; 9] = [
        &[],
        &[(Key::A, true)],
        &[],
        &[(Key::A, false), (Key::A, false)],
        &[(Key::LeftCtrl, true), (Key::A, true)],
        &[(Key::LeftCtrl, false), (Key::A, false)],
        &[],
        &[(Key::ModCtrl, true)],
        &[(Key::ModCtrl, false)],
    ];

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario io_keyboard_events\n");

    for (i, events) in frames.iter().enumerate() {
        ctx.io_mut().delta_time = 0.1;
        for (key, down) in events.iter() {
            ctx.io_mut().add_key_event(*key, *down);
        }
        ctx.begin_frame();

        let io = ctx.io();
        let a = io.key_data(Key::A);
        let ctrl = io.key_data(Key::LeftCtrl);
        out.push_str(&format!(
            "f{i} t {:08x} a_down {} a_dur {:08x} a_press {} ctrl_down {} ctrl_dur {:08x} ctrl_press {} mods {}{}{}{}\n",
            bits(ctx.time() as f32),
            a.down as u8,
            bits(a.down_duration),
            io.is_key_pressed(Key::A) as u8,
            ctrl.down as u8,
            bits(ctrl.down_duration),
            io.is_key_pressed(Key::LeftCtrl) as u8,
            io.key_ctrl as u8,
            io.key_shift as u8,
            io.key_alt as u8,
            io.key_super as u8,
        ));
        ctx.end_frame();
    }
    out
}

/// The `font_metrics` scenario: the default font's baked metrics, its glyph advances and
/// `CalcTextSize` at two sizes. The advances are pinned inputs (see `Font::reference_default`).
pub fn font_metrics_dump() -> String {
    use oxidized_imgui::Font;

    let font = Font::reference_default();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario font_metrics\n");
    out.push_str(&format!(
        "font size {:08x} ascent {:08x} descent {:08x}\n",
        bits(font.size),
        bits(font.ascent),
        bits(font.descent)
    ));

    for c in ['H', 'i', 'e', 'l', 'o'] {
        let advance = font.glyph(c).map(|g| g.advance_x).unwrap_or(0.0);
        out.push_str(&format!("glyph '{c}' advance {:08x}\n", bits(advance)));
    }

    let texts: [(&str, f32); 4] = [("", 13.0), ("Hello", 13.0), ("Hello", 26.0), ("Hi", 13.0)];
    for (text, size) in texts {
        let measured = font.calc_text_size(text, size);
        out.push_str(&format!(
            "text \"{text}\" at {:08x} size {:08x} {:08x}\n",
            bits(size),
            bits(measured.x),
            bits(measured.y)
        ));
    }
    out
}

/// The `clip_stack` scenario: the clip stack of a draw list and the command splits it causes.
pub fn clip_stack_dump() -> String {
    use oxidized_imgui::{DrawList, Vec2};

    fn dump_list(out: &mut String, name: &str, list: &DrawList) {
        out.push_str(&format!(
            "{name} cmds {} vtx {} idx {}\n",
            list.cmd_buffer.len(),
            list.vtx_buffer.len(),
            list.idx_buffer.len()
        ));
        for (i, cmd) in list.cmd_buffer.iter().enumerate() {
            out.push_str(&format!(
                "  cmd {i} clip {:08x} {:08x} {:08x} {:08x} elems {}\n",
                bits(cmd.clip_rect[0]),
                bits(cmd.clip_rect[1]),
                bits(cmd.clip_rect[2]),
                bits(cmd.clip_rect[3]),
                cmd.elem_count
            ));
        }
    }

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario clip_stack\n");

    let mut simple = DrawList::new();
    simple.white_uv = WHITE_UV;
    simple.push_clip_rect(
        Vec2 { x: 70.0, y: 70.0 },
        Vec2 { x: 170.0, y: 170.0 },
        false,
    );
    simple.prim_rect(Vec2 { x: 60.0, y: 60.0 }, Vec2 { x: 200.0, y: 200.0 }, COL);
    simple.pop_clip_rect();
    simple.prim_rect(
        Vec2 { x: 210.0, y: 210.0 },
        Vec2 { x: 250.0, y: 250.0 },
        COL,
    );
    dump_list(&mut out, "clip_simple", &simple);

    let mut intersect = DrawList::new();
    intersect.white_uv = WHITE_UV;
    intersect.push_clip_rect(
        Vec2 { x: 60.0, y: 60.0 },
        Vec2 { x: 200.0, y: 200.0 },
        false,
    );
    intersect.push_clip_rect(Vec2 { x: 70.0, y: 70.0 }, Vec2 { x: 300.0, y: 300.0 }, true);
    intersect.prim_rect(Vec2 { x: 65.0, y: 65.0 }, Vec2 { x: 290.0, y: 290.0 }, COL);
    intersect.pop_clip_rect();
    intersect.prim_rect(Vec2 { x: 65.0, y: 65.0 }, Vec2 { x: 190.0, y: 190.0 }, COL);
    intersect.pop_clip_rect();
    dump_list(&mut out, "clip_intersect", &intersect);

    out
}

/// The `text_draw` scenario: glyph quads for a short string.
///
/// UVs are **omitted on purpose**: the atlas coordinates depend on the reference's internal
/// packer and differ between runs of the same driver, so they cannot be a stable contract. What is
/// compared is the geometry (positions, colours, indices), which is what text layout decides.
pub fn text_draw_dump() -> String {
    use oxidized_imgui::{DrawList, Font, Vec2};

    let font = Font::reference_default();
    let mut list = DrawList::new();
    list.white_uv = WHITE_UV;
    list.add_text(&font, 13.0, Vec2 { x: 10.0, y: 20.0 }, 0xFFFF_FFFF, "Hi");

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario text_draw\n");
    out.push_str("uv omitted: atlas packing is not reproducible across runs\n");
    out.push_str(&format!("cmd_count {}\n", list.cmd_buffer.len()));
    for (i, cmd) in list.cmd_buffer.iter().enumerate() {
        out.push_str(&format!(
            "cmd {i} clip {:08x} {:08x} {:08x} {:08x} elems {}\n",
            bits(cmd.clip_rect[0]),
            bits(cmd.clip_rect[1]),
            bits(cmd.clip_rect[2]),
            bits(cmd.clip_rect[3]),
            cmd.elem_count
        ));
    }
    out.push_str(&format!("vtx_count {}\n", list.vtx_buffer.len()));
    for (i, v) in list.vtx_buffer.iter().enumerate() {
        out.push_str(&format!(
            "vtx {i} pos {:08x} {:08x} col {:08x}\n",
            bits(v.pos.x),
            bits(v.pos.y),
            v.col
        ));
    }
    out.push_str(&format!("idx_count {}\n", list.idx_buffer.len()));
    for idx in &list.idx_buffer {
        out.push_str(&format!("idx {idx}\n"));
    }
    out
}

/// The dump for a scenario, whichever family it belongs to.
pub fn dump_for(name: &str) -> Option<String> {
    match name {
        "window_layout" => Some(window_layout_dump()),
        "io_mouse_events" => Some(io_mouse_dump()),
        "io_keyboard_events" => Some(io_keyboard_dump()),
        "font_metrics" => Some(font_metrics_dump()),
        "clip_stack" => Some(clip_stack_dump()),
        "text_draw" => Some(text_draw_dump()),
        _ => scene(name).map(|list| dump(name, &list)),
    }
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
            assert!(dump_for(name).is_some(), "{name}");
        }
        for name in ["prim_rect_uv", "prim_rect", "rect_filled", "line_h"] {
            let text = dump_for(name).expect(name);
            assert!(text.contains("vtx_count 4"), "{name}");
            assert!(text.contains("idx_count 6"), "{name}");
        }
        assert!(
            dump_for("window_layout")
                .expect("core")
                .contains("w0 item1 max")
        );
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
