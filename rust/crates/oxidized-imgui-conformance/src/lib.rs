//! Conformance scenarios and the text dump both implementations must produce.
//!
//! The dump is the contract: `tools/conformance/driver.cpp` prints it from the C++ reference and
//! `tests/golden/` stores that output. The Rust side must reproduce it byte for byte.
//!
//! Floats are printed as the hexadecimal bit pattern, never as decimal text: it makes the diff
//! exact and immune to formatting differences between `printf` and `format!`.

use oxidized_imgui::WindowFlags;
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
    "button",
    "widgets_basic",
    "slider_basic",
    "style_defaults",
    "aa_rect",
    "aa_round_rect",
    "aa_line",
    "aa_strokes",
    "aa_bevel",
    "arc_steps",
    "bezier_points",
    "aa_polyline",
    "aa_border",
    "button_draw",
    "window_bg",
    "window_border",
    "window_title",
    "window_icons",
    "window_icons_states",
    "window_icons_drag",
    "windows_order",
    "windows_move",
    "window_scroll",
    "window_scrollbar",
    "window_scrollbar_drag",
    "window_wheel",
    "window_autosize",
    "window_autosize_change",
    "window_wheel_lock",
    "window_hscrollbar",
    "window_scrollbar_page",
    "window_min_size",
    "window_resize_grips",
    "window_resize_drag",
    "cmd_offsets",
    "scene_window_button",
    "button_states",
    "button_border",
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
        // Closed path with the inside stroke: a one-pixel border around a rectangle.
        "aa_border" => {
            list.white_uv = WHITE_UV;
            list.fringe_scale = 1.0;
            list.add_rect_border_inside_aa(
                Vec2 { x: 10.5, y: 10.25 },
                Vec2 { x: 60.5, y: 45.75 },
                COL,
                1.0,
            );
        }
        "aa_polyline" => {
            list.white_uv = WHITE_UV;
            list.fringe_scale = 1.0;
            let points = [
                Vec2 { x: 10.5, y: 10.25 },
                Vec2 { x: 60.5, y: 45.75 },
                Vec2 { x: 120.25, y: 20.5 },
            ];
            list.add_polyline_aa_miter(&points, COL, 3.0);
        }
        "aa_bevel" => {
            list.white_uv = WHITE_UV;
            list.fringe_scale = 1.0;
            let points = [
                Vec2 { x: 10.5, y: 50.25 },
                Vec2 { x: 200.5, y: 20.5 },
                Vec2 { x: 200.5, y: 80.5 },
            ];
            list.add_convex_poly_filled_aa(&points, COL);
        }
        "aa_line" => {
            list.white_uv = WHITE_UV;
            list.fringe_scale = 1.0;
            list.add_line_aa(
                Vec2 { x: 10.0, y: 10.0 },
                Vec2 { x: 50.0, y: 30.0 },
                COL,
                1.0,
            );
            list.add_line_aa(
                Vec2 { x: 10.0, y: 10.0 },
                Vec2 { x: 50.0, y: 30.0 },
                COL,
                2.0,
            );
        }
        "aa_rect" => {
            list.white_uv = WHITE_UV;
            list.fringe_scale = 1.0;
            list.add_rect_filled_aa(Vec2 { x: 10.5, y: 20.25 }, Vec2 { x: 60.5, y: 50.75 }, COL);
        }
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
    use oxidized_imgui::{Font, Vec2};

    // The atlas is only stable inside the process that baked it, so the table travels with the
    // scenario as a fixture written by the same reference run that produced the golden.
    const GLYPHS: &str = include_str!("../tests/fixtures/text_draw.glyphs.txt");
    let font = Font::from_pinned(GLYPHS).expect("glyph fixture");

    let mut list = DrawList::new();
    list.white_uv = WHITE_UV;
    list.fringe_scale = 1.0;
    list.add_text(&font, 13.0, Vec2 { x: 10.0, y: 20.0 }, 0xFFFF_FFFF, "Hi");
    dump("text_draw", &list)
}
/// The `button` scenario: one button, six frames of mouse input, printing the item rect, the
/// hover/active state, the return value and the cursor afterwards.
pub fn button_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let frames: [(f32, f32, bool); 6] = [
        (0.0, 0.0, false),
        (96.0, 89.0, false),
        (96.0, 89.0, true),
        (96.0, 89.0, true),
        (96.0, 89.0, false),
        (96.0, 89.0, false),
    ];

    let mut ctx = Context::new();
    // The reference runs one frame before the loop so its font is available.
    ctx.begin_frame();
    ctx.end_frame();

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario button\n");

    for (i, (x, y, down)) in frames.iter().enumerate() {
        {
            let io = ctx.io_mut();
            io.delta_time = 0.1;
            io.mouse_pos = Some(Vec2 { x: *x, y: *y });
            io.mouse_down[0] = *down;
        }
        ctx.begin_frame();
        ctx.begin_window(
            "W",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        ctx.set_cursor_screen_pos(Vec2 { x: 80.0, y: 80.0 });
        let ret = ctx.button("Hi");
        let min = ctx.item_rect_min();
        let max = ctx.item_rect_max();
        let cursor = ctx.cursor_screen_pos();
        out.push_str(&format!(
            "f{i} pos {:08x} {:08x} down {} rect {:08x} {:08x} {:08x} {:08x} hov {} act {} ret {} cursor {:08x} {:08x}\n",
            bits(*x),
            bits(*y),
            *down as u8,
            bits(min.x),
            bits(min.y),
            bits(max.x),
            bits(max.y),
            ctx.last_item_hovered() as u8,
            ctx.last_item_active() as u8,
            ret as u8,
            bits(cursor.x),
            bits(cursor.y),
        ));
        ctx.end_window();
        ctx.end_frame();
    }
    out
}

/// The `widgets_basic` scenario: a label's item rect and a checkbox over six frames.
pub fn widgets_basic_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    ctx.begin_frame();
    ctx.end_frame();

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario widgets_basic\n");

    // Label.
    {
        let io = ctx.io_mut();
        io.delta_time = 0.1;
        io.mouse_pos = Some(Vec2 { x: 0.0, y: 0.0 });
        io.mouse_down[0] = false;
    }
    ctx.begin_frame();
    ctx.begin_window(
        "W",
        Some(Vec2 { x: 60.0, y: 60.0 }),
        Some(Vec2 { x: 300.0, y: 200.0 }),
    );
    ctx.set_cursor_screen_pos(Vec2 { x: 80.0, y: 80.0 });
    ctx.label("Hi");
    let min = ctx.item_rect_min();
    let max = ctx.item_rect_max();
    let cursor = ctx.cursor_screen_pos();
    out.push_str(&format!(
        "label rect {:08x} {:08x} {:08x} {:08x} cursor {:08x} {:08x}\n",
        bits(min.x),
        bits(min.y),
        bits(max.x),
        bits(max.y),
        bits(cursor.x),
        bits(cursor.y)
    ));
    ctx.end_window();
    ctx.end_frame();

    // Checkbox.
    let frames: [(f32, f32, bool); 6] = [
        (0.0, 0.0, false),
        (98.0, 89.0, false),
        (98.0, 89.0, true),
        (98.0, 89.0, false),
        (0.0, 0.0, false),
        (98.0, 89.0, true),
    ];
    let mut value = false;
    for (i, (x, y, down)) in frames.iter().enumerate() {
        {
            let io = ctx.io_mut();
            io.delta_time = 0.1;
            io.mouse_pos = Some(Vec2 { x: *x, y: *y });
            io.mouse_down[0] = *down;
        }
        ctx.begin_frame();
        ctx.begin_window(
            "W",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        ctx.set_cursor_screen_pos(Vec2 { x: 80.0, y: 80.0 });
        let ret = ctx.checkbox("Hi", &mut value);
        let min = ctx.item_rect_min();
        let max = ctx.item_rect_max();
        let cursor = ctx.cursor_screen_pos();
        out.push_str(&format!(
            "cb{i} rect {:08x} {:08x} {:08x} {:08x} hov {} act {} v {} ret {} cursor {:08x} {:08x}\n",
            bits(min.x),
            bits(min.y),
            bits(max.x),
            bits(max.y),
            ctx.last_item_hovered() as u8,
            ctx.last_item_active() as u8,
            value as u8,
            ret as u8,
            bits(cursor.x),
            bits(cursor.y),
        ));
        ctx.end_window();
        ctx.end_frame();
    }
    out
}

/// The `slider_basic` scenario: a float slider pressed, dragged and released.
pub fn slider_basic_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let frames: [(f32, f32, bool); 5] = [
        (0.0, 0.0, false),
        (130.0, 129.0, false),
        (130.0, 129.0, true),
        (150.0, 129.0, true),
        (150.0, 129.0, false),
    ];

    let mut ctx = Context::new();
    ctx.begin_frame();
    ctx.end_frame();

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario slider_basic\n");

    let mut value = 5.0f32;
    for (i, (x, y, down)) in frames.iter().enumerate() {
        {
            let io = ctx.io_mut();
            io.delta_time = 0.1;
            io.mouse_pos = Some(Vec2 { x: *x, y: *y });
            io.mouse_down[0] = *down;
        }
        ctx.begin_frame();
        ctx.begin_window(
            "W",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        ctx.set_cursor_screen_pos(Vec2 { x: 80.0, y: 120.0 });
        let ret = ctx.slider_float("S", &mut value, 0.0, 10.0, 100.0);
        let min = ctx.item_rect_min();
        let max = ctx.item_rect_max();
        let cursor = ctx.cursor_screen_pos();
        out.push_str(&format!(
            "sl{i} rect {:08x} {:08x} {:08x} {:08x} hov {} act {} v {:08x} ret {} cursor {:08x} {:08x}\n",
            bits(min.x),
            bits(min.y),
            bits(max.x),
            bits(max.y),
            ctx.last_item_hovered() as u8,
            ctx.last_item_active() as u8,
            bits(value),
            ret as u8,
            bits(cursor.x),
            bits(cursor.y),
        ));
        ctx.end_window();
        ctx.end_frame();
    }
    out
}

/// The `style_defaults` scenario: the default style's colours, roundings and border sizes — the
/// value layer of appearance.
pub fn style_defaults_dump() -> String {
    use oxidized_imgui::{Col, Style};

    let style = Style::default();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario style_defaults\n");

    let entries: [(&str, Col); 17] = [
        ("Text", Col::Text),
        ("TextDisabled", Col::TextDisabled),
        ("WindowBg", Col::WindowBg),
        ("FrameBg", Col::FrameBg),
        ("FrameBgHovered", Col::FrameBgHovered),
        ("FrameBgActive", Col::FrameBgActive),
        ("Button", Col::Button),
        ("ButtonHovered", Col::ButtonHovered),
        ("ButtonActive", Col::ButtonActive),
        ("CheckMark", Col::CheckMark),
        ("SliderGrab", Col::SliderGrab),
        ("SliderGrabActive", Col::SliderGrabActive),
        ("Border", Col::Border),
        ("Separator", Col::Separator),
        ("Header", Col::Header),
        ("HeaderHovered", Col::HeaderHovered),
        ("HeaderActive", Col::HeaderActive),
    ];
    for (name, col) in entries {
        out.push_str(&format!("col {name} {:08x}\n", style.color_u32(col)));
    }

    out.push_str(&format!(
        "round frame {:08x} window {:08x} grab {:08x} child {:08x} popup {:08x} scrollbar {:08x} tab {:08x}\n",
        bits(style.frame_rounding),
        bits(style.window_rounding),
        bits(style.grab_rounding),
        bits(style.child_rounding),
        bits(style.popup_rounding),
        bits(style.scrollbar_rounding),
        bits(style.tab_rounding),
    ));
    out.push_str(&format!(
        "border window {:08x} frame {:08x} child {:08x} popup {:08x}\n",
        bits(style.window_border_size),
        bits(style.frame_border_size),
        bits(style.child_border_size),
        bits(style.popup_border_size),
    ));
    out
}

/// The `aa_round_rect` scenario: a rounded rectangle, antialiased. Prints the corner arc points
/// and then the full draw list. `arc_step` is pinned: the reference's automatic segment count is
/// not ported yet.
pub fn aa_round_rect_dump() -> String {
    use oxidized_imgui::{DrawList, Vec2};

    let min = Vec2 { x: 10.5, y: 20.25 };
    let max = Vec2 { x: 60.5, y: 50.75 };

    let points = DrawList::path_rect_rounded(min, max, 5.0);
    let mut list = DrawList::new();
    list.white_uv = WHITE_UV;
    list.fringe_scale = 1.0;
    list.add_rect_filled_aa_rounded(min, max, 5.0, COL);

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario aa_round_rect\n");
    for (i, p) in points.iter().enumerate() {
        out.push_str(&format!("p {i} {:08x} {:08x}\n", bits(p.x), bits(p.y)));
    }
    let full = dump("aa_round_rect", &list);
    for line in full.lines().skip(2) {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The `arc_steps` scenario: the reference's automatic corner sampling for a range of radii, as the
/// number of points it produces for a rounded rectangle and the step that follows from it.
pub fn arc_steps_dump() -> String {
    use oxidized_imgui::{DrawList, Vec2};

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario arc_steps\n");
    for radius in 1..=14u32 {
        let points = DrawList::path_rect_rounded(
            Vec2 { x: 10.5, y: 20.25 },
            Vec2 {
                x: 200.5,
                y: 150.75,
            },
            radius as f32,
        );
        let per_corner = points.len() / 4;
        let step = 12 / (per_corner as i32 - 1);
        out.push_str(&format!(
            "r {radius} rect_pts {} step {step}\n",
            points.len()
        ));
    }
    out
}

/// The `bezier_points` scenario: the auto-tessellated points of a cubic bezier. Pure arithmetic, so
/// this is the one curve that can be compared bit for bit.
pub fn bezier_points_dump() -> String {
    use oxidized_imgui::Vec2;

    let mut path = vec![Vec2 { x: 10.0, y: 10.0 }];
    oxidized_imgui::path_bezier_cubic_to(
        &mut path,
        Vec2 { x: 50.0, y: 90.0 },
        Vec2 { x: 130.0, y: -20.0 },
        Vec2 { x: 180.0, y: 60.0 },
        1.12,
        1.0,
    );

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario bezier_points\n");
    out.push_str(&format!("n {}\n", path.len()));
    for (i, p) in path.iter().enumerate() {
        out.push_str(&format!("p {i} {:08x} {:08x}\n", bits(p.x), bits(p.y)));
    }
    out
}

/// The `button_draw` scenario: the draw list of a frame holding a single button, in a window
/// without background so the list contains only the widget. The glyph table travels as a fixture
/// because the atlas is only stable inside the reference process.
pub fn button_draw_dump() -> String {
    use oxidized_imgui::{Context, Font, Vec2};

    const GLYPHS: &str = include_str!("../tests/fixtures/button_draw.glyphs.txt");
    let font = Font::from_pinned(GLYPHS).expect("glyph fixture");

    let mut ctx = Context::new();
    ctx.begin_frame();
    ctx.begin_window_ex(
        "W",
        Some(Vec2 { x: 60.0, y: 60.0 }),
        Some(Vec2 { x: 300.0, y: 200.0 }),
        false,
    );
    ctx.set_cursor_screen_pos(Vec2 { x: 80.0, y: 80.0 });
    ctx.set_font_for_drawing(font);
    let _ = ctx.button("Hi");
    let out = dump("button_draw", ctx.draw_list());
    ctx.end_window();
    ctx.end_frame();
    out
}

/// The `window_bg` scenario: a window background with no border.
pub fn window_bg_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    ctx.style_mut().window_border_size = 0.0;
    ctx.begin_frame();
    ctx.begin_window(
        "W",
        Some(Vec2 { x: 60.0, y: 60.0 }),
        Some(Vec2 { x: 300.0, y: 200.0 }),
    );
    ctx.end_window();
    dump("window_bg", ctx.draw_list())
}

/// The `window_border` scenario: a window with its background and one-pixel border.
pub fn window_border_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    ctx.begin_frame();
    ctx.begin_window(
        "W",
        Some(Vec2 { x: 60.0, y: 60.0 }),
        Some(Vec2 { x: 300.0, y: 200.0 }),
    );
    ctx.end_window();
    dump("window_border", ctx.draw_list())
}

/// The `window_title` scenario: a titled window, so the title band and its text are in the list.
pub fn window_title_dump() -> String {
    use oxidized_imgui::{Context, Font, Vec2};

    // Read at run time: the fixture is written by the reference in the same run as the golden, and
    // an atlas re-bake can move a glyph by a pixel, so a compile-time copy could be stale.
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/window_title.glyphs.txt"
    );
    let glyphs = std::fs::read_to_string(path).expect("glyph fixture");
    let font = Font::from_pinned(&glyphs).expect("glyph fixture");

    let mut ctx = Context::new();
    ctx.set_font_for_drawing(font);
    ctx.begin_frame();
    ctx.begin_window_titled(
        "W",
        "W",
        Some(Vec2 { x: 60.0, y: 60.0 }),
        Some(Vec2 { x: 300.0, y: 200.0 }),
    );
    ctx.end_window();
    dump("window_title", ctx.draw_list())
}

/// The `cmd_offsets` scenario: two commands in one list, so each carries the cumulative vertex and
/// index offsets of the buffers at the moment it opened.
pub fn cmd_offsets_dump() -> String {
    use oxidized_imgui::{DrawList, Vec2};

    let mut list = DrawList::new();
    list.white_uv = WHITE_UV;
    list.fringe_scale = 1.0;
    list.add_rect_filled(
        Vec2 { x: 10.0, y: 10.0 },
        Vec2 { x: 50.0, y: 30.0 },
        0xFF00_00FF,
    );
    list.push_clip_rect(Vec2 { x: 20.0, y: 20.0 }, Vec2 { x: 80.0, y: 60.0 }, false);
    list.add_rect_filled(
        Vec2 { x: 30.0, y: 25.0 },
        Vec2 { x: 70.0, y: 45.0 },
        0xFF00_FF00,
    );
    dump("cmd_offsets", &list)
}
/// The `scene_window_button` scenario: a titled window with a button inside, compared whole. The
/// reference settles for twelve frames first, because a window does not know its title bar height
/// during its first one; the dump is taken after the window closes, pinning the command list too.
pub fn scene_window_button_dump() -> String {
    use oxidized_imgui::{Context, Font, Vec2};

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/scene_window_button.glyphs.txt"
    );
    let glyphs = std::fs::read_to_string(path).expect("glyph fixture");
    let font = Font::from_pinned(&glyphs).expect("glyph fixture");

    let mut ctx = Context::new();
    ctx.set_font_for_drawing(font);
    ctx.begin_frame();
    ctx.begin_window_titled(
        "W",
        "W",
        Some(Vec2 { x: 60.0, y: 60.0 }),
        Some(Vec2 { x: 300.0, y: 200.0 }),
    );
    let _ = ctx.button("Hi");
    ctx.end_window();
    let out = dump("scene_window_button", ctx.draw_list());
    ctx.end_frame();
    out
}
/// The `button_states` scenario: the same button resting, hovered and held down. The frames run in
/// order so the press registers, and each state is dumped right after its frame.
pub fn button_states_dump() -> String {
    use oxidized_imgui::{Context, Font, Vec2};

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/button_states.glyphs.txt"
    );
    let glyphs = std::fs::read_to_string(path).expect("glyph fixture");
    let font = Font::from_pinned(&glyphs).expect("glyph fixture");

    let mut ctx = Context::new();
    ctx.set_font_for_drawing(font);

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario button_states\n");

    let run = |ctx: &mut Context, label: &str, mx: f32, my: f32, down: bool, out: &mut String| {
        ctx.io_mut().mouse_pos = Some(Vec2 { x: mx, y: my });
        ctx.io_mut().mouse_down[0] = down;
        ctx.begin_frame();
        ctx.begin_window_ex(
            "W",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
            false,
        );
        ctx.set_cursor_screen_pos(Vec2 { x: 80.0, y: 80.0 });
        let _ = ctx.button("Hi");
        // The reference's list is dumped after the window closes, which is why it carries the
        // trailing empty command the clip pop opens.
        ctx.end_window();
        let section = dump(label, ctx.draw_list());
        out.push_str(&format!("state {label}\n"));
        for line in section.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    run(&mut ctx, "resting", 0.0, 0.0, false, &mut out);
    run(&mut ctx, "hovered", 85.0, 85.0, false, &mut out);
    run(&mut ctx, "active", 85.0, 85.0, true, &mut out);
    out
}
/// The `button_border` scenario: the button with `FrameBorderSize` set, so its frame has an outline.
pub fn button_border_dump() -> String {
    use oxidized_imgui::{Context, Font, Vec2};

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/button_border.glyphs.txt"
    );
    let glyphs = std::fs::read_to_string(path).expect("glyph fixture");
    let font = Font::from_pinned(&glyphs).expect("glyph fixture");

    let mut ctx = Context::new();
    ctx.set_font_for_drawing(font);
    ctx.style_mut().frame_border_size = 1.0;
    ctx.io_mut().mouse_pos = Some(Vec2 { x: 0.0, y: 0.0 });
    ctx.begin_frame();
    ctx.begin_window_ex(
        "W",
        Some(Vec2 { x: 60.0, y: 60.0 }),
        Some(Vec2 { x: 300.0, y: 200.0 }),
        false,
    );
    ctx.set_cursor_screen_pos(Vec2 { x: 80.0, y: 80.0 });
    let _ = ctx.button("Hi");
    ctx.end_window();
    let out = dump("button_border", ctx.draw_list());
    ctx.end_frame();
    out
}
/// The `aa_strokes` scenario: the anti-aliased stroke path **with the atlas line textures**, which
/// is the path a real window's draw list takes. Three lines of different thickness, an open
/// polyline with a soft join, the inside stroke with anti-aliased ends that a window icon uses, a
/// corner sharp enough to bevel, and a closed inside stroke.
///
/// The line table is a pinned input: the atlas packer decides where the rows land, so the fixture
/// is written by the same reference run that produced the golden.
pub fn aa_strokes_dump() -> String {
    use oxidized_imgui::{DrawList, Vec2, draw_flags};

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/aa_strokes.lineuvs.txt"
    );
    let text = std::fs::read_to_string(path).expect("line uv fixture");

    let mut list = DrawList::new();
    list.white_uv = WHITE_UV;
    list.fringe_scale = 1.0;
    list.tex_uv_lines = parse_line_uvs(&text);
    // `ImDrawListSharedData::InitialDrawFlags`: the defaults a window's draw list starts with.
    list.flags = draw_flags::AALINES
        | draw_flags::AAFILL
        | draw_flags::USE_TEX_FOR_ROUND_CORNERS
        | draw_flags::ALLOW_TEX_FOR_ROUND_CORNERS;

    let p = |x: f32, y: f32| Vec2 { x, y };

    list.add_line(p(10.0, 10.0), p(50.0, 30.0), COL, 1.0, 0);
    list.add_line(p(10.0, 40.0), p(50.0, 60.0), COL, 2.0, 0);
    list.add_line(p(10.0, 70.0), p(50.0, 90.0), COL, 4.0, 0);

    let points = [p(70.5, 10.25), p(120.5, 45.75), p(180.25, 20.5)];
    list.add_polyline(&points, COL, 3.0, 0);

    // The shape a window's collapse arrow takes: inside stroke, anti-aliased ends, soft join.
    let arrow = [p(70.5, 120.25), p(75.5, 125.75), p(80.5, 120.25)];
    list.add_polyline(
        &arrow,
        COL,
        2.0,
        draw_flags::STROKE_INSIDE | draw_flags::AALINE_ENDS,
    );

    // A corner sharp enough that the miter limit turns the join into a clipped bevel.
    let sharp = [p(10.5, 200.25), p(200.5, 200.75), p(10.5, 205.25)];
    list.add_polyline(
        &sharp,
        COL,
        2.0,
        draw_flags::STROKE_INSIDE | draw_flags::AALINE_ENDS,
    );

    list.add_rect_border_inside_aa(p(100.5, 100.25), p(180.5, 150.75), COL, 2.0);

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario aa_strokes\n");
    out.push_str(&format!("list_flags {:08x}\n", list.flags));
    let full = dump("aa_strokes", &list);
    for line in full.lines().skip(2) {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Reads the atlas line-texture table the reference wrote: one `line <n> <u0> <u1> <v> <1/width>`
/// per baked thickness, as raw float bits. Rows of the other tables a fixture may carry (the font
/// header and the glyphs) are ignored, so one file can hold both.
fn parse_line_uvs(text: &str) -> Vec<[f32; 4]> {
    let mut table: Vec<[f32; 4]> = vec![[0.0; 4]; 46];
    for line in text.lines().filter(|l| l.starts_with("line ")) {
        let mut fields = line.split_whitespace();
        assert_eq!(fields.next(), Some("line"), "unexpected line fixture row");
        let index: usize = fields.next().expect("index").parse().expect("line index");
        let mut entry = [0.0f32; 4];
        for slot in entry.iter_mut() {
            let bits = u32::from_str_radix(fields.next().expect("value"), 16).expect("hex bits");
            *slot = f32::from_bits(bits);
        }
        table[index] = entry;
    }
    table
}

/// The `window_icons` scenario: a titled window that carries both window icons — a close cross
/// because it is closable, a collapse arrow because it is collapsible — dumped whole.
///
/// The title is text and the icons are strokes, so the fixture carries both pinned tables: the
/// glyphs and the atlas line rows.
pub fn window_icons_dump() -> String {
    use oxidized_imgui::{Context, Font, Vec2};

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/window_icons.fixture.txt"
    );
    let text = std::fs::read_to_string(path).expect("window icons fixture");
    let font = Font::from_pinned(&text).expect("font fixture");
    let line_uvs = parse_line_uvs(&text);

    let mut ctx = Context::new();
    ctx.set_font_for_drawing(font);
    ctx.set_line_uv_lines(line_uvs);

    let mut open = true;
    // The reference settles first: a window does not know its title bar height during its first
    // frame. The port computes it directly, but runs the same frames so both sides are compared
    // after the same history.
    let mut out = String::new();
    for frame in 0..14 {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: 0.0, y: 0.0 });
        ctx.io_mut().mouse_down[0] = false;
        ctx.begin_frame();
        ctx.begin_window_titled_with_icons(
            "W",
            "W",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
            Some(&mut open),
            true,
        );
        ctx.end_window();
        if frame == 13 {
            // Dumped after the window closes, like the other window scenarios.
            out = dump("window_icons", ctx.draw_list());
        }
        ctx.end_frame();
    }
    out
}

/// The `window_icons_states` scenario: the same window hovered, held, released inside and outside,
/// collapsed and expanded. Each state is one dumped section taken right after its frame, and each
/// prints the flags the frame produced: `open`, `collapsed` and what `Begin` returned.
pub fn window_icons_states_dump() -> String {
    use oxidized_imgui::{Context, Font, Vec2};

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/window_icons_states.fixture.txt"
    );
    let text = std::fs::read_to_string(path).expect("window icons fixture");
    let font = Font::from_pinned(&text).expect("font fixture");
    let line_uvs = parse_line_uvs(&text);

    let mut ctx = Context::new();
    ctx.set_font_for_drawing(font);
    ctx.set_line_uv_lines(line_uvs);
    let mut open = true;

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_icons_states\n");

    // Settle, then one extra frame that wires the atlas tables, like the reference does.
    for _ in 0..14 {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: 0.0, y: 0.0 });
        ctx.io_mut().mouse_down[0] = false;
        ctx.begin_frame();
        ctx.begin_window_titled_with_icons(
            "W",
            "W",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
            Some(&mut open),
            true,
        );
        ctx.end_window();
        ctx.end_frame();
    }

    // The two icon squares: the arrow at (65, 63) and the cross at (342, 63), 13x13 each.
    let (arrow_x, arrow_y) = (70.0f32, 69.0f32);
    let (close_x, close_y) = (348.0f32, 69.0f32);

    let s = |ctx: &mut oxidized_imgui::Context,
             out: &mut String,
             open: &mut bool,
             name: &str,
             mx: f32,
             my: f32,
             down: bool| {
        icons_state_section(ctx, out, open, name, mx, my, down);
    };

    s(&mut ctx, &mut out, &mut open, "resting", 0.0, 0.0, false);
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "hover_arrow",
        arrow_x,
        arrow_y,
        false,
    );
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "hover_close",
        close_x,
        close_y,
        false,
    );
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "held_close",
        close_x,
        close_y,
        true,
    );
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "released_close",
        close_x,
        close_y,
        false,
    );

    // The same press, released away from the button: it must not close.
    open = true;
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "held_close_again",
        close_x,
        close_y,
        true,
    );
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "released_outside",
        100.0,
        150.0,
        false,
    );

    // Collapse with the arrow, then expand again.
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "held_arrow",
        arrow_x,
        arrow_y,
        true,
    );
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "released_arrow",
        arrow_x,
        arrow_y,
        false,
    );
    s(&mut ctx, &mut out, &mut open, "collapsed", 0.0, 0.0, false);
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "collapsed_hover_arrow",
        arrow_x,
        arrow_y,
        false,
    );
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "held_arrow_collapsed",
        arrow_x,
        arrow_y,
        true,
    );
    s(
        &mut ctx,
        &mut out,
        &mut open,
        "released_arrow_collapsed",
        arrow_x,
        arrow_y,
        false,
    );
    s(&mut ctx, &mut out, &mut open, "expanded", 0.0, 0.0, false);
    out
}

/// One frame of the `window_icons_states` scenario, dumped with the flags it produced.
fn icons_state_section(
    ctx: &mut oxidized_imgui::Context,
    out: &mut String,
    open: &mut bool,
    name: &str,
    mx: f32,
    my: f32,
    down: bool,
) {
    use oxidized_imgui::Vec2;

    ctx.io_mut().delta_time = 0.1;
    ctx.io_mut().mouse_pos = Some(Vec2 { x: mx, y: my });
    ctx.io_mut().mouse_down[0] = down;
    ctx.begin_frame();
    let began = ctx.begin_window_titled_with_icons(
        "W",
        "W",
        Some(Vec2 { x: 60.0, y: 60.0 }),
        Some(Vec2 { x: 300.0, y: 200.0 }),
        Some(open),
        true,
    );
    let collapsed = ctx.window().collapsed;
    ctx.end_window();
    out.push_str(&format!(
        "state {name} open {} collapsed {} began {}\n",
        *open as u8, collapsed as u8, began as u8
    ));
    // The reference dumps right after the frame, before the next one resets the list.
    let body = dump("window_icons_states", ctx.draw_list());
    for line in body.lines().skip(2) {
        out.push_str(line);
        out.push('\n');
    }
    ctx.end_frame();
}

/// The `window_icons_drag` scenario: the mouse grabs the collapse arrow and drags past the
/// threshold, which starts moving the window (the reference's `CollapseButton` calls
/// `StartMouseMovingWindow`). Each state prints the window position, so a mismatch says where the
/// window actually was.
///
/// The window is movable here — no `ImGuiWindowFlags_NoMove` — and its position is only requested
/// on the first frame, because asking for it every frame would override the drag.
pub fn window_icons_drag_dump() -> String {
    use oxidized_imgui::{Context, Font, Vec2};

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/window_icons_drag.fixture.txt"
    );
    let text = std::fs::read_to_string(path).expect("window icons fixture");
    let font = Font::from_pinned(&text).expect("font fixture");
    let line_uvs = parse_line_uvs(&text);

    let mut ctx = Context::new();
    ctx.set_font_for_drawing(font);
    ctx.set_line_uv_lines(line_uvs);
    let mut open = true;

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_icons_drag\n");

    let frame = |ctx: &mut Context, out: &mut String, open: &mut bool, first: bool| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: 0.0, y: 0.0 });
        ctx.io_mut().mouse_down[0] = false;
        ctx.begin_frame();
        let pos = if first {
            Some(Vec2 { x: 60.0, y: 60.0 })
        } else {
            None
        };
        let size = if first {
            Some(Vec2 { x: 300.0, y: 200.0 })
        } else {
            None
        };
        ctx.begin_window_titled_with_icons("W", "W", pos, size, Some(open), false);
        ctx.end_window();
        ctx.end_frame();
        let _ = out;
    };

    // Settle: the first frame places the window, the rest keep it where it is.
    frame(&mut ctx, &mut out, &mut open, true);
    for _ in 0..13 {
        frame(&mut ctx, &mut out, &mut open, false);
    }

    let section = |ctx: &mut Context,
                   out: &mut String,
                   open: &mut bool,
                   name: &str,
                   mx: f32,
                   my: f32,
                   down: bool| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: mx, y: my });
        ctx.io_mut().mouse_down[0] = down;
        ctx.begin_frame();
        let began = ctx.begin_window_titled_with_icons("W", "W", None, None, Some(open), false);
        let pos = ctx.window().pos;
        let collapsed = ctx.window().collapsed;
        ctx.end_window();
        out.push_str(&format!(
            "state {name} pos {:08x} {:08x} open {} collapsed {} began {} down {}\n",
            pos.x.to_bits(),
            pos.y.to_bits(),
            *open as u8,
            collapsed as u8,
            began as u8,
            down as u8
        ));
        let body = dump("window_icons_drag", ctx.draw_list());
        for line in body.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    section(&mut ctx, &mut out, &mut open, "idle", 0.0, 0.0, false);
    section(
        &mut ctx,
        &mut out,
        &mut open,
        "hover_arrow",
        70.0,
        69.0,
        false,
    );
    section(&mut ctx, &mut out, &mut open, "press", 70.0, 69.0, true);
    section(
        &mut ctx,
        &mut out,
        &mut open,
        "below_threshold",
        74.0,
        70.0,
        true,
    );
    section(
        &mut ctx,
        &mut out,
        &mut open,
        "drag_start",
        90.0,
        90.0,
        true,
    );
    section(&mut ctx, &mut out, &mut open, "dragged", 110.0, 100.0, true);
    section(
        &mut ctx, &mut out, &mut open, "released", 110.0, 100.0, false,
    );
    section(
        &mut ctx,
        &mut out,
        &mut open,
        "after_release",
        0.0,
        0.0,
        false,
    );
    out
}

/// The `windows_order` scenario: two overlapping titled windows and what a click does to the order
/// they are rendered in.
///
/// The dump is the renderer's view — the `ImDrawList`s `GetDrawData()` would hand the backend, in
/// order — plus the window the mouse is over. The click's own frame already shows the raised order,
/// because the reference raises before it builds the draw data.
pub fn windows_order_dump() -> String {
    use oxidized_imgui::{Context, Font, Vec2};

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/windows_order.glyphs.txt"
    );
    let glyphs = std::fs::read_to_string(path).expect("glyph fixture");
    let font = Font::from_pinned(&glyphs).expect("glyph fixture");

    let mut ctx = Context::new();
    ctx.set_font_for_drawing(font);

    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario windows_order\n");

    let frame =
        |ctx: &mut Context, out: &mut String, n: usize, tag: &str, mx: f32, my: f32, down: bool| {
            ctx.io_mut().delta_time = 0.1;
            ctx.io_mut().mouse_pos = Some(Vec2 { x: mx, y: my });
            ctx.io_mut().mouse_down[0] = down;
            ctx.begin_frame();
            ctx.begin_window_titled(
                "A",
                "A",
                Some(Vec2 { x: 60.0, y: 60.0 }),
                Some(Vec2 { x: 300.0, y: 200.0 }),
            );
            ctx.end_window();
            ctx.begin_window_titled(
                "B",
                "B",
                Some(Vec2 { x: 110.0, y: 100.0 }),
                Some(Vec2 { x: 300.0, y: 200.0 }),
            );
            ctx.end_window();
            let hovered = ctx.hovered_window_name().unwrap_or("-").to_string();
            ctx.end_frame();
            let lists = ctx.render_lists();
            out.push_str(&format!(
                "frame {n} {tag} hovered {hovered} lists {}\n",
                lists.len()
            ));
            for (i, (name, list)) in lists.iter().enumerate() {
                out.push_str(&format!("list {i} {name}\n"));
                let body = dump("windows_order", list);
                for line in body.lines().skip(2) {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        };

    for _ in 0..6 {
        frame(&mut ctx, &mut out, 0, "settle", 0.0, 0.0, false);
    }
    out.clear();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario windows_order\n");

    frame(&mut ctx, &mut out, 0, "idle", 0.0, 0.0, false);
    frame(&mut ctx, &mut out, 1, "press_a_only", 70.0, 70.0, true);
    frame(&mut ctx, &mut out, 2, "release_a_only", 70.0, 70.0, false);
    frame(&mut ctx, &mut out, 3, "idle_a_on_top", 0.0, 0.0, false);
    frame(&mut ctx, &mut out, 4, "press_both", 200.0, 150.0, true);
    frame(&mut ctx, &mut out, 5, "release_both", 200.0, 150.0, false);
    frame(&mut ctx, &mut out, 6, "press_b_only", 400.0, 290.0, true);
    frame(&mut ctx, &mut out, 7, "release_b_only", 400.0, 290.0, false);
    frame(&mut ctx, &mut out, 8, "idle_b_on_top", 0.0, 0.0, false);
    out
}

/// The `windows_move` scenario: dragging one window by its title bar and by its body, and the clamp
/// that keeps it reachable when it is dragged off the display.
///
/// Only positions are compared: a window's geometry is what `windows_order` already pins.
pub fn windows_move_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario windows_move\n");

    let frame = |ctx: &mut Context,
                 out: &mut String,
                 tag: &str,
                 mx: f32,
                 my: f32,
                 down: bool,
                 first: bool| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: mx, y: my });
        ctx.io_mut().mouse_down[0] = down;
        ctx.begin_frame();
        let pos = if first {
            Some(Vec2 { x: 60.0, y: 60.0 })
        } else {
            None
        };
        let size = if first {
            Some(Vec2 { x: 300.0, y: 200.0 })
        } else {
            None
        };
        ctx.begin_window_titled_movable("W", "W", pos, size);
        let window_pos = ctx.window().pos;
        ctx.end_window();
        if !tag.is_empty() {
            out.push_str(&format!(
                "state {tag} pos {:08x} {:08x}\n",
                window_pos.x.to_bits(),
                window_pos.y.to_bits()
            ));
        }
        ctx.end_frame();
    };

    for i in 0..6 {
        frame(&mut ctx, &mut out, "", 0.0, 0.0, false, i == 0);
    }

    // The title bar of W is y 60..79 at x 60..360.
    frame(&mut ctx, &mut out, "settled", 0.0, 0.0, false, false);
    frame(&mut ctx, &mut out, "press_title", 100.0, 65.0, true, false);
    frame(&mut ctx, &mut out, "drag_title_1", 140.0, 90.0, true, false);
    frame(
        &mut ctx,
        &mut out,
        "drag_title_2",
        200.0,
        120.0,
        true,
        false,
    );
    frame(
        &mut ctx,
        &mut out,
        "release_title",
        200.0,
        120.0,
        false,
        false,
    );
    // And from the body: with MoveFromTitleBarOnly off, the whole window moves.
    frame(&mut ctx, &mut out, "press_body", 250.0, 200.0, true, false);
    frame(&mut ctx, &mut out, "drag_body", 280.0, 220.0, true, false);
    frame(
        &mut ctx,
        &mut out,
        "release_body",
        280.0,
        220.0,
        false,
        false,
    );
    // Far up-left: clamped so a corner stays inside the padded work area.
    frame(
        &mut ctx,
        &mut out,
        "press_topleft",
        300.0,
        240.0,
        true,
        false,
    );
    frame(&mut ctx, &mut out, "drag_to_origin", 0.0, 0.0, true, false);
    frame(
        &mut ctx,
        &mut out,
        "drag_off_screen",
        -400.0,
        -400.0,
        true,
        false,
    );
    frame(
        &mut ctx,
        &mut out,
        "release_off_screen",
        -400.0,
        -400.0,
        false,
        false,
    );
    // And far down-right.
    frame(&mut ctx, &mut out, "press_again", 0.0, 0.0, true, false);
    frame(
        &mut ctx,
        &mut out,
        "drag_far_right",
        2000.0,
        2000.0,
        true,
        false,
    );
    frame(
        &mut ctx,
        &mut out,
        "release_far_right",
        2000.0,
        2000.0,
        false,
        false,
    );
    out
}

/// The `window_scroll` scenario: a window without a scrollbar whose content is taller than the
/// window, scrolled by API. The scroll, its maximum and the interior rectangle are printed beside
/// the draw list so a mismatch says which of them moved.
pub fn window_scroll_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_scroll\n");

    let frame = |ctx: &mut Context, out: &mut String, scroll_y: Option<f32>| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: 0.0, y: 0.0 });
        ctx.io_mut().mouse_down[0] = false;
        ctx.begin_frame();
        if let Some(y) = scroll_y {
            ctx.set_next_window_scroll(Vec2 { x: 0.0, y });
        }
        ctx.begin_window(
            "S",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        let start = ctx.cursor_screen_pos();
        ctx.dummy(Vec2 { x: 100.0, y: 300.0 });
        let after = ctx.cursor_screen_pos();
        ctx.draw_list_mut().add_rect_filled(
            Vec2 {
                x: start.x,
                y: after.y,
            },
            Vec2 {
                x: start.x + 50.0,
                y: after.y + 20.0,
            },
            0xFF00_00FF,
        );
        // Dumped before `end_window`, like the reference: the clip pop at End would open one more
        // command.
        let body_text = dump("window_scroll", ctx.draw_list());
        let window = ctx.window();
        let (inner_min, inner_max) = window.inner_rect;
        let scroll_text = format!(
            "frame scroll {:08x} max {:08x} content {:08x} {:08x} inner {:08x} {:08x} {:08x} {:08x} start {:08x} {:08x} after {:08x} {:08x}\n",
            window.scroll.y.to_bits(),
            window.scroll_max.y.to_bits(),
            window.content_extent.x.to_bits(),
            window.content_extent.y.to_bits(),
            inner_min.x.to_bits(),
            inner_min.y.to_bits(),
            inner_max.x.to_bits(),
            inner_max.y.to_bits(),
            start.x.to_bits(),
            start.y.to_bits(),
            after.x.to_bits(),
            after.y.to_bits(),
        );
        ctx.end_window();
        out.push_str(&scroll_text);
        for line in body_text.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    for _ in 0..6 {
        frame(&mut ctx, &mut out, None);
    }
    for _ in 0..2 {
        frame(&mut ctx, &mut out, Some(40.0));
    }
    for _ in 0..2 {
        frame(&mut ctx, &mut out, Some(10000.0));
    }
    out
}

/// The `window_scrollbar` scenario: a window whose content does not fit, so the vertical scrollbar
/// appears and takes its width out of the interior. Drawing only — the mouse stays away, so the
/// grab is neither hovered nor held and cannot be dragged yet.
pub fn window_scrollbar_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_scrollbar\n");

    let frame = |ctx: &mut Context, out: &mut String, scroll_y: Option<f32>| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: 0.0, y: 0.0 });
        ctx.io_mut().mouse_down[0] = false;
        ctx.begin_frame();
        if let Some(y) = scroll_y {
            ctx.set_next_window_scroll(Vec2 { x: 0.0, y });
        }
        ctx.set_next_window_flags(WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE);
        ctx.begin_window(
            "S",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        ctx.dummy(Vec2 { x: 100.0, y: 300.0 });
        let window = ctx.window();
        let (inner_min, inner_max) = window.inner_rect;
        let border = 1.0;
        let bar_min = Vec2 {
            x: window.pos.x + window.size.x - border - if window.scrollbar_y { 14.0 } else { 0.0 },
            y: inner_min.y + border,
        };
        let bar_max = Vec2 {
            x: window.pos.x + window.size.x - border,
            y: inner_max.y - border,
        };
        let text = format!(
            "frame scrollbar {} scroll {:08x} max {:08x} inner {:08x} {:08x} {:08x} {:08x} bar {:08x} {:08x} {:08x} {:08x}\n",
            window.scrollbar_y as u8,
            window.scroll.y.to_bits(),
            window.scroll_max.y.to_bits(),
            inner_min.x.to_bits(),
            inner_min.y.to_bits(),
            inner_max.x.to_bits(),
            inner_max.y.to_bits(),
            bar_min.x.to_bits(),
            bar_min.y.to_bits(),
            bar_max.x.to_bits(),
            bar_max.y.to_bits(),
        );
        let body = dump("window_scrollbar", ctx.draw_list());
        ctx.end_window();
        out.push_str(&text);
        for line in body.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    for _ in 0..3 {
        frame(&mut ctx, &mut out, None);
    }
    for _ in 0..2 {
        frame(&mut ctx, &mut out, Some(40.0));
    }
    out
}

/// The `window_scrollbar_drag` scenario: grabbing the scrollbar and clicking its track.
///
/// The grab sits at the top while the scroll is 0, so the press at y 230 lands below it and seeks
/// straight to the maximum, and the one at y 70 seeks back to the top.
pub fn window_scrollbar_drag_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_scrollbar_drag\n");

    let frame = |ctx: &mut Context, out: &mut String, mx: f32, my: f32, down: bool| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: mx, y: my });
        ctx.io_mut().mouse_down[0] = down;
        ctx.begin_frame();
        ctx.set_next_window_flags(WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE);
        ctx.begin_window(
            "S",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        ctx.dummy(Vec2 { x: 100.0, y: 300.0 });
        let window = ctx.window();
        let text = format!(
            "frame scroll {:08x} max {:08x}\n",
            window.scroll.y.to_bits(),
            window.scroll_max.y.to_bits(),
        );
        let body = dump("window_scrollbar_drag", ctx.draw_list());
        ctx.end_window();
        out.push_str(&text);
        for line in body.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    for _ in 0..4 {
        frame(&mut ctx, &mut out, 0.0, 0.0, false);
    }
    // The bar spans x 345..359; the grab is at the top of it (y 63..185).
    frame(&mut ctx, &mut out, 352.0, 100.0, false);
    frame(&mut ctx, &mut out, 352.0, 100.0, true);
    frame(&mut ctx, &mut out, 352.0, 120.0, true);
    frame(&mut ctx, &mut out, 352.0, 140.0, true);
    frame(&mut ctx, &mut out, 352.0, 140.0, false);
    frame(&mut ctx, &mut out, 0.0, 0.0, false);
    // A click far below the grab: absolute seek to that point.
    frame(&mut ctx, &mut out, 352.0, 230.0, true);
    frame(&mut ctx, &mut out, 352.0, 230.0, false);
    frame(&mut ctx, &mut out, 0.0, 0.0, false);
    // And a click above it, which seeks back towards the top.
    frame(&mut ctx, &mut out, 352.0, 70.0, true);
    frame(&mut ctx, &mut out, 352.0, 70.0, false);
    frame(&mut ctx, &mut out, 0.0, 0.0, false);
    out
}

/// The `window_wheel` scenario: the mouse wheel over a window scrolls it.
///
/// The step is five font sizes (65 pixels here), capped at two thirds of the interior, and it is
/// symmetric: a notch up undoes a notch down.
pub fn window_wheel_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_wheel\n");

    let frame = |ctx: &mut Context, out: &mut String, wheel_y: f32| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: 200.0, y: 150.0 });
        ctx.io_mut().mouse_down[0] = false;
        ctx.io_mut().mouse_wheel = wheel_y;
        ctx.begin_frame();
        ctx.set_next_window_flags(WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE);
        ctx.begin_window(
            "S",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        ctx.dummy(Vec2 { x: 100.0, y: 300.0 });
        let window = ctx.window();
        let text = format!(
            "frame wheel {:08x} scroll {:08x} max {:08x}\n",
            wheel_y.to_bits(),
            window.scroll.y.to_bits(),
            window.scroll_max.y.to_bits(),
        );
        let body = dump("window_wheel", ctx.draw_list());
        ctx.end_window();
        out.push_str(&text);
        for line in body.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    // The mouse sits over the window body, away from the scrollbar.
    for _ in 0..4 {
        frame(&mut ctx, &mut out, 0.0);
    }
    frame(&mut ctx, &mut out, -1.0);
    frame(&mut ctx, &mut out, -1.0);
    frame(&mut ctx, &mut out, 0.0);
    frame(&mut ctx, &mut out, 1.0);
    frame(&mut ctx, &mut out, 1.0);
    frame(&mut ctx, &mut out, 0.5);
    frame(&mut ctx, &mut out, 0.0);
    out
}

/// The `window_autosize` scenario: a window that takes the size its content needs.
///
/// The size is the content of the previous frame plus the window padding (and the title bar when
/// there is one), which is why the first frame shows only the padding: the content is not known yet.
pub fn window_autosize_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_autosize\n");

    let frame = |ctx: &mut Context, out: &mut String| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: 0.0, y: 0.0 });
        ctx.io_mut().mouse_down[0] = false;
        ctx.begin_frame();
        ctx.set_next_window_flags(
            WindowFlags::NO_MOVE
                | WindowFlags::NO_RESIZE
                | WindowFlags::NO_SCROLLBAR
                | WindowFlags::ALWAYS_AUTO_RESIZE,
        );
        ctx.begin_window("A", Some(Vec2 { x: 60.0, y: 60.0 }), None);
        ctx.dummy(Vec2 { x: 100.0, y: 300.0 });
        let window = ctx.window();
        let (inner_min, inner_max) = window.inner_rect;
        let text = format!(
            "frame size {:08x} {:08x} sizefull {:08x} {:08x} inner {:08x} {:08x} {:08x} {:08x} content {:08x} {:08x}\n",
            window.size.x.to_bits(),
            window.size.y.to_bits(),
            window.size_full.x.to_bits(),
            window.size_full.y.to_bits(),
            inner_min.x.to_bits(),
            inner_min.y.to_bits(),
            inner_max.x.to_bits(),
            inner_max.y.to_bits(),
            window.content_extent.x.to_bits(),
            window.content_extent.y.to_bits(),
        );
        let body = dump("window_autosize", ctx.draw_list());
        ctx.end_window();
        out.push_str(&text);
        for line in body.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    for _ in 0..6 {
        frame(&mut ctx, &mut out);
    }
    out
}

/// The `window_autosize_change` scenario: an auto-sized window whose content changes.
///
/// The size always follows the content of the previous frame, so both shrinking and growing show up
/// one frame later. That is `CalcWindowAutoFitSize(window, window->ContentSizeIdeal, ...)`: the ideal
/// content size is measured at the end of a frame and only used by the next one.
pub fn window_autosize_change_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_autosize_change\n");

    let frame = |ctx: &mut Context, out: &mut String, content_h: f32| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: 0.0, y: 0.0 });
        ctx.io_mut().mouse_down[0] = false;
        ctx.begin_frame();
        ctx.set_next_window_flags(
            WindowFlags::NO_MOVE
                | WindowFlags::NO_RESIZE
                | WindowFlags::NO_SCROLLBAR
                | WindowFlags::ALWAYS_AUTO_RESIZE,
        );
        ctx.begin_window("A", Some(Vec2 { x: 60.0, y: 60.0 }), None);
        let window = ctx.window();
        let (inner_min, inner_max) = window.inner_rect;
        let text = format!(
            "frame draw {:08x} size {:08x} {:08x} content {:08x} {:08x} inner {:08x} {:08x} {:08x} {:08x}\n",
            content_h.to_bits(),
            window.size.x.to_bits(),
            window.size.y.to_bits(),
            window.content_extent.x.to_bits(),
            window.content_extent.y.to_bits(),
            inner_min.x.to_bits(),
            inner_min.y.to_bits(),
            inner_max.x.to_bits(),
            inner_max.y.to_bits(),
        );
        let body = dump("window_autosize_change", ctx.draw_list());
        out.push_str(&text);
        for line in body.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.dummy(Vec2 {
            x: 100.0,
            y: content_h,
        });
        ctx.end_window();
        ctx.end_frame();
    };

    frame(&mut ctx, &mut out, 300.0);
    frame(&mut ctx, &mut out, 300.0);
    // The content shrinks: the size follows one frame later.
    frame(&mut ctx, &mut out, 50.0);
    frame(&mut ctx, &mut out, 50.0);
    // And grows: same one-frame lag.
    frame(&mut ctx, &mut out, 200.0);
    frame(&mut ctx, &mut out, 200.0);
    out
}

/// The `window_wheel_lock` scenario: the window the wheel is locked to keeps scrolling even when the
/// mouse moves over another one, until it moves past the drag threshold.
///
/// A = (60,60)-(360,260) and B = (358,60)-(658,260) overlap by two pixels, so a four pixel move takes
/// the mouse from A alone to a point inside both, where B is on top and therefore hovered.
pub fn window_wheel_lock_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_wheel_lock\n");

    let frame = |ctx: &mut Context, out: &mut String, mx: f32, my: f32, wheel_y: f32| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: mx, y: my });
        ctx.io_mut().mouse_down[0] = false;
        ctx.io_mut().mouse_wheel = wheel_y;
        ctx.begin_frame();
        ctx.set_next_window_flags(WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE);
        ctx.begin_window(
            "A",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        let scroll_a = ctx.window().scroll.y;
        ctx.dummy(Vec2 { x: 100.0, y: 300.0 });
        ctx.end_window();
        ctx.set_next_window_flags(WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE);
        ctx.begin_window(
            "B",
            Some(Vec2 { x: 358.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        let scroll_b = ctx.window().scroll.y;
        ctx.dummy(Vec2 { x: 100.0, y: 300.0 });
        ctx.end_window();
        let hovered = ctx.hovered_window_name().unwrap_or("-").to_string();
        let locked = ctx.wheeling_window_name().unwrap_or("-").to_string();
        out.push_str(&format!(
            "frame wheel {:08x} a {:08x} b {:08x} hovered {hovered} locked {locked}\n",
            wheel_y.to_bits(),
            scroll_a.to_bits(),
            scroll_b.to_bits(),
        ));
        ctx.end_frame();
    };

    // Settle with the mouse over A alone (x 355 is left of B's edge at 358).
    for _ in 0..4 {
        frame(&mut ctx, &mut out, 355.0, 150.0, 0.0);
    }
    // A notch over A.
    frame(&mut ctx, &mut out, 355.0, 150.0, -1.0);
    // A four pixel move puts the mouse inside both, where B is on top and hovered.
    frame(&mut ctx, &mut out, 359.0, 150.0, 0.0);
    // The lock keeps the wheel on A even though B is hovered now.
    frame(&mut ctx, &mut out, 359.0, 150.0, -1.0);
    frame(&mut ctx, &mut out, 359.0, 150.0, 0.0);
    // A big move takes the mouse to B alone: the lock is released and B scrolls.
    frame(&mut ctx, &mut out, 500.0, 150.0, 0.0);
    frame(&mut ctx, &mut out, 500.0, 150.0, -1.0);
    frame(&mut ctx, &mut out, 500.0, 150.0, 0.0);
    out
}

/// The `window_hscrollbar` scenario: a window whose content is wider than itself, so the horizontal
/// scrollbar appears, takes its height off the bottom of the interior, and can be dragged.
pub fn window_hscrollbar_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_hscrollbar\n");

    let frame = |ctx: &mut Context,
                 out: &mut String,
                 scroll_x: Option<f32>,
                 mx: f32,
                 my: f32,
                 down: bool| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: mx, y: my });
        ctx.io_mut().mouse_down[0] = down;
        ctx.begin_frame();
        if let Some(x) = scroll_x {
            ctx.set_next_window_scroll(Vec2 { x, y: 0.0 });
        }
        ctx.set_next_window_flags(
            WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE | WindowFlags::HORIZONTAL_SCROLLBAR,
        );
        ctx.begin_window(
            "H",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        ctx.dummy(Vec2 { x: 400.0, y: 100.0 });
        let window = ctx.window();
        let (inner_min, inner_max) = window.inner_rect;
        let border = 1.0;
        let bar_min = Vec2 {
            x: inner_min.x + border,
            y: window.pos.y + window.size.y - border - if window.scrollbar_x { 14.0 } else { 0.0 },
        };
        let bar_max = Vec2 {
            x: inner_max.x - border,
            y: window.pos.y + window.size.y - border,
        };
        let text = format!(
            "frame scroll {:08x} max {:08x} {:08x} inner {:08x} {:08x} {:08x} {:08x} bar {:08x} {:08x} {:08x} {:08x} sb {} {}\n",
            window.scroll.x.to_bits(),
            window.scroll_max.x.to_bits(),
            window.scroll_max.y.to_bits(),
            inner_min.x.to_bits(),
            inner_min.y.to_bits(),
            inner_max.x.to_bits(),
            inner_max.y.to_bits(),
            bar_min.x.to_bits(),
            bar_min.y.to_bits(),
            bar_max.x.to_bits(),
            bar_max.y.to_bits(),
            window.scrollbar_x as u8,
            window.scrollbar_y as u8,
        );
        let body = dump("window_hscrollbar", ctx.draw_list());
        ctx.end_window();
        out.push_str(&text);
        for line in body.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    // No content size on the first frame, so the bar appears on the second.
    frame(&mut ctx, &mut out, None, 0.0, 0.0, false);
    frame(&mut ctx, &mut out, None, 0.0, 0.0, false);
    // Scrolled by API: the grab moves right.
    frame(&mut ctx, &mut out, Some(60.0), 0.0, 0.0, false);
    // Grab the bar and drag it, then release.
    frame(&mut ctx, &mut out, None, 100.0, 253.0, false);
    frame(&mut ctx, &mut out, None, 100.0, 253.0, true);
    frame(&mut ctx, &mut out, None, 140.0, 253.0, true);
    frame(&mut ctx, &mut out, None, 140.0, 253.0, false);
    frame(&mut ctx, &mut out, None, 0.0, 0.0, false);
    out
}

/// The `window_scrollbar_page` scenario: `ConfigScrollbarScrollByPage` makes a click on the bar's
/// track scroll a page instead of jumping to the clicked point, and Shift forces the absolute seek
/// back even with the option on.
///
/// The content is 1000 tall in a 200 tall window, so the grab is short (38 px out of 194) and a click
/// below it lands far from the bottom: a page is 200, the absolute seek something else entirely.
pub fn window_scrollbar_page_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_scrollbar_page\n");

    let frame = |ctx: &mut Context,
                 out: &mut String,
                 scroll_y: Option<f32>,
                 by_page: bool,
                 shift: bool,
                 my: f32,
                 down: bool| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: 352.0, y: my });
        ctx.io_mut().mouse_down[0] = down;
        ctx.io_mut().config_scrollbar_scroll_by_page = by_page;
        ctx.begin_frame();
        // `begin_frame` rebuilds the keyboard state, so Shift has to be set after it.
        ctx.io_mut().key_shift = shift;
        if let Some(y) = scroll_y {
            ctx.set_next_window_scroll(Vec2 { x: 0.0, y });
        }
        ctx.set_next_window_flags(WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE);
        ctx.begin_window(
            "P",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        ctx.dummy(Vec2 {
            x: 100.0,
            y: 1000.0,
        });
        let window = ctx.window();
        let text = format!(
            "frame scroll {:08x} max {:08x}\n",
            window.scroll.y.to_bits(),
            window.scroll_max.y.to_bits(),
        );
        let body = dump("window_scrollbar_page", ctx.draw_list());
        ctx.end_window();
        out.push_str(&text);
        for line in body.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    for _ in 0..4 {
        frame(&mut ctx, &mut out, None, false, false, 0.0, false);
    }
    // By page, click below the grab: one page down (200), not the absolute seek.
    frame(&mut ctx, &mut out, None, true, false, 200.0, false);
    frame(&mut ctx, &mut out, None, true, false, 200.0, true);
    frame(&mut ctx, &mut out, None, true, false, 200.0, false);
    frame(&mut ctx, &mut out, None, true, false, 200.0, false);
    // Again from the same place: another page (400).
    frame(&mut ctx, &mut out, None, true, false, 200.0, true);
    frame(&mut ctx, &mut out, None, true, false, 200.0, false);
    // Shift forces the absolute seek even with the option on.
    frame(&mut ctx, &mut out, None, true, true, 200.0, false);
    frame(&mut ctx, &mut out, None, true, true, 200.0, true);
    frame(&mut ctx, &mut out, None, true, true, 200.0, false);
    // Option off: the click seeks to where it landed.
    frame(&mut ctx, &mut out, Some(0.0), false, false, 200.0, false);
    frame(&mut ctx, &mut out, Some(0.0), false, false, 200.0, true);
    frame(&mut ctx, &mut out, Some(0.0), false, false, 200.0, false);
    out
}

/// The `window_min_size` scenario: `WindowMinSize` clamps a size asked for by API, while an
/// auto-sized window uses the 1 pixel hard minimum instead and can end up smaller than the style.
pub fn window_min_size_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_min_size\n");

    let frame =
        |ctx: &mut Context, out: &mut String, w: f32, h: f32, auto: bool, dump_frame: bool| {
            ctx.io_mut().delta_time = 0.1;
            ctx.io_mut().mouse_pos = Some(Vec2 { x: 0.0, y: 0.0 });
            ctx.io_mut().mouse_down[0] = false;
            ctx.begin_frame();
            if auto {
                // An auto-sized window ignores the requested size, because the size was set by API.
                ctx.set_next_window_flags(
                    WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE | WindowFlags::ALWAYS_AUTO_RESIZE,
                );
            }
            ctx.begin_window(
                "M",
                Some(Vec2 { x: 60.0, y: 60.0 }),
                Some(Vec2 { x: w, y: h }),
            );
            if auto {
                ctx.dummy(Vec2 { x: 1.0, y: 1.0 });
            }
            let window = ctx.window();
            if !dump_frame {
                ctx.end_window();
                ctx.end_frame();
                return;
            }
            let text = format!(
                "frame requested {:08x} {:08x} auto {} size {:08x} {:08x} min {:08x} {:08x}\n",
                w.to_bits(),
                h.to_bits(),
                auto as u8,
                window.size.x.to_bits(),
                window.size.y.to_bits(),
                ctx.style().window_min_size.x.to_bits(),
                ctx.style().window_min_size.y.to_bits(),
            );
            let body = dump("window_min_size", ctx.draw_list());
            ctx.end_window();
            out.push_str(&text);
            for line in body.lines().skip(2) {
                out.push_str(line);
                out.push('\n');
            }
            ctx.end_frame();
        };

    // A tiny requested size is bumped to the minimum.
    frame(&mut ctx, &mut out, 10.0, 10.0, false, true);
    frame(&mut ctx, &mut out, 10.0, 10.0, false, true);
    // Just under: also bumped.
    frame(&mut ctx, &mut out, 31.0, 31.0, false, true);
    // Exactly at the minimum: unchanged.
    frame(&mut ctx, &mut out, 32.0, 32.0, false, true);
    // Above it: unchanged.
    frame(&mut ctx, &mut out, 120.0, 80.0, false, true);
    frame(&mut ctx, &mut out, 10.0, 10.0, true, true);
    out
}

/// The `window_resize_grips` scenario: the lower corner grips of a resizable window. The lower-right
/// one is visible at rest, the lower-left one only while it is hovered or held, and the colours
/// change with the mouse. Resizing itself is not wired up yet, so only the drawing is pinned.
pub fn window_resize_grips_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_resize_grips\n");

    let frame = |ctx: &mut Context, out: &mut String, mx: f32, my: f32, down: bool| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: mx, y: my });
        ctx.io_mut().mouse_down[0] = down;
        ctx.begin_frame();
        ctx.set_next_window_flags(WindowFlags::NO_MOVE | WindowFlags::NO_SCROLLBAR);
        ctx.begin_window(
            "G",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        let window = ctx.window();
        let text = format!(
            "frame size {:08x} {:08x}\n",
            window.size.x.to_bits(),
            window.size.y.to_bits(),
        );
        let body = dump("window_resize_grips", ctx.draw_list());
        ctx.end_window();
        out.push_str(&text);
        for line in body.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    // Away from the window: only the lower-right grip is visible, faintly.
    for _ in 0..3 {
        frame(&mut ctx, &mut out, 0.0, 0.0, false);
    }
    // Over the lower-right grip (the window ends at 360,260; the handle reaches 12 pixels out).
    frame(&mut ctx, &mut out, 357.0, 257.0, false);
    frame(&mut ctx, &mut out, 357.0, 257.0, true);
    frame(&mut ctx, &mut out, 357.0, 257.0, false);
    // Over the lower-left grip: the second one shows up too.
    frame(&mut ctx, &mut out, 63.0, 257.0, false);
    frame(&mut ctx, &mut out, 0.0, 0.0, false);
    out
}

/// The `window_resize_drag` scenario: dragging the lower-right resize grip changes the window size.
///
/// The size comes from the mouse position and the offset inside the handle, so a drag is a delta; the
/// scenario asks for the same size every frame, which is why the size springs back on release.
pub fn window_resize_drag_dump() -> String {
    use oxidized_imgui::{Context, Vec2};

    let mut ctx = Context::new();
    let mut out = String::new();
    out.push_str(&format!(
        "header imgui {IMGUI_VERSION} {IMGUI_VERSION_NUM}\n"
    ));
    out.push_str("scenario window_resize_drag\n");

    let frame = |ctx: &mut Context, out: &mut String, mx: f32, my: f32, down: bool| {
        ctx.io_mut().delta_time = 0.1;
        ctx.io_mut().mouse_pos = Some(Vec2 { x: mx, y: my });
        ctx.io_mut().mouse_down[0] = down;
        ctx.begin_frame();
        ctx.set_next_window_flags(WindowFlags::NO_MOVE | WindowFlags::NO_SCROLLBAR);
        ctx.begin_window(
            "R",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        let window = ctx.window();
        let text = format!(
            "frame size {:08x} {:08x} pos {:08x} {:08x}\n",
            window.size.x.to_bits(),
            window.size.y.to_bits(),
            window.pos.x.to_bits(),
            window.pos.y.to_bits(),
        );
        let body = dump("window_resize_drag", ctx.draw_list());
        ctx.end_window();
        out.push_str(&text);
        for line in body.lines().skip(2) {
            out.push_str(line);
            out.push('\n');
        }
        ctx.end_frame();
    };

    for _ in 0..4 {
        frame(&mut ctx, &mut out, 0.0, 0.0, false);
    }
    // The window ends at (360, 260); the handle reaches 12 pixels out and 10 in.
    frame(&mut ctx, &mut out, 357.0, 257.0, false);
    frame(&mut ctx, &mut out, 357.0, 257.0, true);
    frame(&mut ctx, &mut out, 377.0, 277.0, true);
    frame(&mut ctx, &mut out, 397.0, 297.0, true);
    frame(&mut ctx, &mut out, 397.0, 297.0, false);
    frame(&mut ctx, &mut out, 0.0, 0.0, false);
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
        "button" => Some(button_dump()),
        "widgets_basic" => Some(widgets_basic_dump()),
        "slider_basic" => Some(slider_basic_dump()),
        "style_defaults" => Some(style_defaults_dump()),
        "aa_round_rect" => Some(aa_round_rect_dump()),
        "aa_strokes" => Some(aa_strokes_dump()),
        "window_bg" => Some(window_bg_dump()),
        "window_border" => Some(window_border_dump()),
        "window_title" => Some(window_title_dump()),
        "window_icons" => Some(window_icons_dump()),
        "window_icons_states" => Some(window_icons_states_dump()),
        "window_icons_drag" => Some(window_icons_drag_dump()),
        "windows_order" => Some(windows_order_dump()),
        "windows_move" => Some(windows_move_dump()),
        "window_scroll" => Some(window_scroll_dump()),
        "window_scrollbar" => Some(window_scrollbar_dump()),
        "window_scrollbar_drag" => Some(window_scrollbar_drag_dump()),
        "window_wheel" => Some(window_wheel_dump()),
        "window_autosize" => Some(window_autosize_dump()),
        "window_autosize_change" => Some(window_autosize_change_dump()),
        "window_wheel_lock" => Some(window_wheel_lock_dump()),
        "window_hscrollbar" => Some(window_hscrollbar_dump()),
        "window_scrollbar_page" => Some(window_scrollbar_page_dump()),
        "window_min_size" => Some(window_min_size_dump()),
        "window_resize_grips" => Some(window_resize_grips_dump()),
        "window_resize_drag" => Some(window_resize_drag_dump()),
        "cmd_offsets" => Some(cmd_offsets_dump()),
        "scene_window_button" => Some(scene_window_button_dump()),
        "button_states" => Some(button_states_dump()),
        "button_border" => Some(button_border_dump()),
        "button_draw" => Some(button_draw_dump()),
        "arc_steps" => Some(arc_steps_dump()),
        "bezier_points" => Some(bezier_points_dump()),
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
