//! Context, windows and cursor layout.
//!
//! The part of the reference that decides where things go: a window with an explicit position and
//! size, its content region, and a cursor advancing over items (`Dummy`, `SameLine`). Text
//! metrics, widgets, scrolling, stacking and settings are not implemented yet.
//!
//! Numbers here are verified against the reference by the `window_layout` conformance scenario.

use crate::draw_list::Vec2;
use crate::font::Font;
use crate::id::{ImGuiID, hash_str_utf8};
use crate::io::{BUTTONS, Io, Key};
use crate::style::Style;

/// An open window. Only the geometry the harness checks is modelled.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub name: String,
    pub pos: Vec2,
    pub size: Vec2,
    /// `pos + style.WindowPadding`.
    pub content_origin: Vec2,
    /// `size - style.WindowPadding * 2`, what `GetContentRegionAvail` returns inside the window.
    pub content_size: Vec2,
    cursor: Vec2,
    line_start_x: f32,
    line_height: f32,
    last_min: Vec2,
    last_max: Vec2,
    has_item: bool,
}

/// Frame state, identifier stack, style, input state and the open window stack.
#[derive(Debug)]
pub struct Context {
    id_stack: Vec<ImGuiID>,
    frame_count: u64,
    last_frame_draw_calls: usize,
    style: Style,
    font: Font,
    io: Io,
    /// `g.Time`: the reference keeps it as a `double` and `GetTime()` returns it as such.
    time: f64,
    windows: Vec<Window>,
    remembered: Vec<(String, Vec2, Vec2)>,
    active_id: Option<ImGuiID>,
    /// The window's draw list: what this frame has drawn so far.
    draw_list: crate::draw_list::DrawList,
    slider_just_activated: bool,
    slider_grab_offset: f32,
    last_item_hovered: bool,
    last_item_active: bool,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            id_stack: Vec::new(),
            frame_count: 0,
            last_frame_draw_calls: 0,
            style: Style::default(),
            font: Font::reference_default(),
            io: Io::default(),
            time: 0.0,
            windows: Vec::new(),
            remembered: Vec::new(),
            active_id: None,
            draw_list: crate::draw_list::DrawList::new(),
            slider_just_activated: false,
            slider_grab_offset: 0.0,
            last_item_hovered: false,
            last_item_active: false,
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            style: Style::default(),
            ..Self::default()
        }
    }

    pub fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }

    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Starts a frame: advances the clock, turns the raw input into events, and (as the reference
    /// also does) leaves the identifier stack untouched.
    pub fn begin_frame(&mut self) {
        self.frame_count += 1;
        self.draw_list = crate::draw_list::DrawList::new();
        self.time += self.io.delta_time as f64;
        self.update_keyboard_inputs();
        self.update_mouse_inputs();
    }

    /// Ends a frame.
    pub fn end_frame(&mut self) {}

    /// `ImGuiIO` for reading the computed state.
    pub fn io(&self) -> &Io {
        &self.io
    }

    /// `ImGuiIO` for feeding raw input before the next `begin_frame`.
    pub fn io_mut(&mut self) -> &mut Io {
        &mut self.io
    }

    /// `g.Time`: seconds accumulated from `DeltaTime`, kept as the reference's `double`.
    /// Replaces the font used for drawing, e.g. with one built from a pinned glyph table.
    pub fn set_font_for_drawing(&mut self, font: crate::font::Font) {
        self.font = font;
    }

    /// The draw list for the current frame.
    pub fn draw_list(&self) -> &crate::draw_list::DrawList {
        &self.draw_list
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    /// The reference's `UpdateInputEvents` (key events only) followed by `UpdateKeyboardInputs`:
    /// queued events are applied, then durations and the modifier flags are derived.
    fn update_keyboard_inputs(&mut self) {
        let queued = core::mem::take(&mut self.io.key_queue);
        for (key, down) in queued {
            let data = &mut self.io.keys[key.index()];
            data.down = down;
            data.analog_value = if down { 1.0 } else { 0.0 };
        }

        let delta_time = self.io.delta_time;
        for data in &mut self.io.keys {
            data.down_duration = if data.down {
                if data.down_duration < 0.0 {
                    0.0
                } else {
                    data.down_duration + delta_time
                }
            } else {
                -1.0
            };
        }

        let keys = &self.io.keys;
        self.io.key_ctrl = keys[Key::ModCtrl.index()].down;
        self.io.key_shift = keys[Key::ModShift.index()].down;
        self.io.key_alt = keys[Key::ModAlt.index()].down;
        self.io.key_super = keys[Key::ModSuper.index()].down;
    }

    /// The reference's `UpdateMouseInputs`, restricted to position, buttons and double clicks.
    fn update_mouse_inputs(&mut self) {
        let time = self.time;
        let delta_time = self.io.delta_time;

        // The reference rounds the position to avoid spreading sub-pixel values.
        if let Some(pos) = self.io.mouse_pos {
            self.io.mouse_pos = Some(Vec2 {
                x: pos.x.floor(),
                y: pos.y.floor(),
            });
        }

        self.io.mouse_delta = match (self.io.mouse_pos, self.io.mouse_pos_prev) {
            (Some(current), Some(previous)) => Vec2 {
                x: current.x - previous.x,
                y: current.y - previous.y,
            },
            _ => Vec2::default(),
        };

        for i in 0..BUTTONS {
            self.io.mouse_clicked[i] =
                self.io.mouse_down[i] && self.io.mouse_down_duration[i] < 0.0;
            self.io.mouse_released[i] =
                !self.io.mouse_down[i] && self.io.mouse_down_duration[i] >= 0.0;
            self.io.mouse_clicked_count[i] = 0;

            let duration = self.io.mouse_down_duration[i];
            self.io.mouse_down_duration[i] = if self.io.mouse_down[i] {
                if duration < 0.0 {
                    0.0
                } else {
                    duration + delta_time
                }
            } else {
                -1.0
            };

            if self.io.mouse_clicked[i] {
                let mut repeated = false;
                // The reference casts the difference to float before comparing, and both the
                // clock and the timeout are floats: keep that, it decides double clicks.
                let elapsed = (time - self.io.mouse_clicked_time[i]) as f32;
                if elapsed < self.io.mouse_double_click_time {
                    if let Some(pos) = self.io.mouse_pos {
                        let dx = pos.x - self.io.mouse_clicked_pos[i].x;
                        let dy = pos.y - self.io.mouse_clicked_pos[i].y;
                        let max = self.io.mouse_double_click_max_dist;
                        if dx * dx + dy * dy < max * max {
                            repeated = true;
                        }
                    }
                }
                self.io.mouse_clicked_last_count[i] = if repeated {
                    self.io.mouse_clicked_last_count[i] + 1
                } else {
                    1
                };
                self.io.mouse_clicked_time[i] = time;
                self.io.mouse_clicked_pos[i] = self.io.mouse_pos.unwrap_or_default();
                self.io.mouse_clicked_count[i] = self.io.mouse_clicked_last_count[i];
            }
        }

        self.io.mouse_pos_prev = self.io.mouse_pos;
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn last_frame_draw_calls(&self) -> usize {
        self.last_frame_draw_calls
    }

    /// Identifier of the current scope, `0` at the root like the reference's window-less case.
    pub fn current_id(&self) -> ImGuiID {
        self.id_stack.last().copied().unwrap_or(0)
    }

    /// Hash of `label` in the current scope (`ImGui::GetID`).
    pub fn get_id(&self, label: &str) -> ImGuiID {
        hash_str_utf8(label, self.current_id())
    }

    /// Opens a scope seeded by `label` (`ImGui::PushID`).
    pub fn push_id(&mut self, label: &str) {
        let id = self.get_id(label);
        self.id_stack.push(id);
    }

    /// Closes the innermost scope (`ImGui::PopID`).
    pub fn pop_id(&mut self) {
        self.id_stack.pop();
    }

    pub fn id_stack_depth(&self) -> usize {
        self.id_stack.len()
    }

    /// `ImGui::Begin` with `SetNextWindowPos`/`SetNextWindowSize` already applied.
    ///
    /// `None` keeps the position or size the window had in a previous frame, as the reference does.
    /// Starts a window and paints its background, like `ImGui::Begin` with default flags.
    pub fn begin_window(&mut self, name: &str, pos: Option<Vec2>, size: Option<Vec2>) {
        self.begin_window_ex(name, pos, size, true);
    }

    /// Starts a window, choosing whether the background is painted
    /// (`ImGuiWindowFlags_NoBackground` when false).
    pub fn begin_window_ex(
        &mut self,
        name: &str,
        pos: Option<Vec2>,
        size: Option<Vec2>,
        background: bool,
    ) {
        let remembered = self
            .remembered
            .iter()
            .rev()
            .find(|(known, _, _)| known == name)
            .map(|(_, p, s)| (*p, *s));
        let (pos, size) = match (pos, size, remembered) {
            (Some(p), Some(s), _) => (p, s),
            (p, s, Some((rp, rs))) => (p.unwrap_or(rp), s.unwrap_or(rs)),
            (p, s, None) => (p.unwrap_or_default(), s.unwrap_or_default()),
        };

        let pad = self.style.window_padding;
        let content_origin = Vec2 {
            x: pos.x + pad.x,
            y: pos.y + pad.y,
        };
        // The background is painted before the window clip is applied, with the fullscreen clip.
        if background {
            self.draw_list
                .set_current_clip(self.draw_list.fullscreen_clip);
            self.draw_list.prim_rect(
                pos,
                Vec2 {
                    x: pos.x + size.x,
                    y: pos.y + size.y,
                },
                self.style.color_u32(crate::style::Col::WindowBg),
            );
        }

        // The window border, inside the rectangle. A pixel-aligned window takes the two-ring frame;
        // fractional coordinates would take the miter stroke, which is not wired up here.
        // The reference draws the border only when the window has a background.
        if background && self.style.window_border_size > 0.0 {
            self.draw_list.add_aligned_rect_border(
                pos,
                Vec2 {
                    x: pos.x + size.x,
                    y: pos.y + size.y,
                },
                self.style.color_u32(crate::style::Col::Border),
                self.style.window_border_size,
            );
        }

        // The window's draw list is clipped to its rectangle, inset by the window border.
        self.draw_list.push_clip_rect(
            Vec2 {
                x: pos.x + self.style.window_border_size,
                y: pos.y + self.style.window_border_size,
            },
            Vec2 {
                x: pos.x + size.x - self.style.window_border_size,
                y: pos.y + size.y - self.style.window_border_size,
            },
            false,
        );
        let content_size = Vec2 {
            x: size.x - pad.x * 2.0,
            y: size.y - pad.y * 2.0,
        };
        self.windows.push(Window {
            name: name.to_string(),
            pos,
            size,
            content_origin,
            content_size,
            cursor: content_origin,
            line_start_x: content_origin.x,
            line_height: 0.0,
            last_min: Vec2::default(),
            last_max: Vec2::default(),
            has_item: false,
        });
    }

    /// `ImGui::End`: the window remembers its position and size for the next frame.
    pub fn end_window(&mut self) {
        self.draw_list.drop_empty_trailing_commands();
        if let Some(window) = self.windows.pop() {
            let entry = (window.name.clone(), window.pos, window.size);
            match self
                .remembered
                .iter_mut()
                .find(|(name, _, _)| *name == window.name)
            {
                Some(slot) => *slot = entry,
                None => self.remembered.push(entry),
            }
        }
    }

    pub fn window(&self) -> &Window {
        self.windows.last().expect("no window is open")
    }

    /// `GetContentRegionAvail` inside the current window.
    pub fn content_avail(&self) -> Vec2 {
        self.window().content_size
    }

    /// `GetCursorScreenPos`.
    pub fn cursor_screen_pos(&self) -> Vec2 {
        self.window().cursor
    }

    pub fn item_rect_min(&self) -> Vec2 {
        self.window().last_min
    }

    pub fn item_rect_max(&self) -> Vec2 {
        self.window().last_max
    }

    /// `ImGui::Dummy`: an item of a given size that advances the cursor.
    pub fn dummy(&mut self, size: Vec2) {
        let spacing = self.style.item_spacing;
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        let min = window.cursor;
        let max = Vec2 {
            x: min.x + size.x,
            y: min.y + size.y,
        };
        // An item placed at the line start begins a new line: the line height starts over.
        if min.x == window.line_start_x {
            window.line_height = 0.0;
        }
        window.last_min = min;
        window.last_max = max;
        window.has_item = true;
        window.line_height = window.line_height.max(size.y);
        // The line ends here: the cursor moves below it and back to the line start.
        window.cursor = Vec2 {
            x: window.line_start_x,
            y: min.y + window.line_height + spacing.y,
        };
    }

    /// `ImGui::SameLine`: the next item goes to the right of the previous one.
    pub fn same_line(&mut self) {
        let spacing = self.style.item_spacing;
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        if window.has_item {
            window.cursor = Vec2 {
                x: window.last_max.x + spacing.x,
                y: window.last_min.y,
            };
        }
    }

    /// `SetCursorScreenPos`: place the next item at an absolute position.
    pub fn set_cursor_screen_pos(&mut self, pos: Vec2) {
        if let Some(window) = self.windows.last_mut() {
            window.cursor = pos;
        }
    }

    pub fn last_item_hovered(&self) -> bool {
        self.last_item_hovered
    }

    pub fn last_item_active(&self) -> bool {
        self.last_item_active
    }

    /// `GetFrameHeight`: the height of a framed widget.
    fn frame_height(&self) -> f32 {
        self.font.size + self.style.frame_padding.y * 2.0
    }

    /// The hover/active/press logic shared by framed widgets (`ButtonBehavior`).
    fn interact(&mut self, id: ImGuiID, size: Vec2) -> bool {
        let (min, window_pos, window_size) = {
            let window = self.window();
            (window.cursor, window.pos, window.size)
        };
        let max = Vec2 {
            x: min.x + size.x,
            y: min.y + size.y,
        };

        let over_window = self
            .io
            .mouse_pos
            .map(|m| {
                m.x >= window_pos.x
                    && m.x < window_pos.x + window_size.x
                    && m.y >= window_pos.y
                    && m.y < window_pos.y + window_size.y
            })
            .unwrap_or(false);
        let hovered = over_window
            && self
                .io
                .mouse_pos
                .map(|m| m.x >= min.x && m.x < max.x && m.y >= min.y && m.y < max.y)
                .unwrap_or(false);

        if hovered && self.io.mouse_clicked[0] {
            self.active_id = Some(id);
        }

        // On release the reference clears the active id before returning, so `IsItemActive()`
        // queried afterwards is already false on that frame.
        let was_active = self.active_id == Some(id);
        let mut pressed = false;
        if was_active && self.io.mouse_released[0] {
            pressed = hovered;
            self.active_id = None;
        }
        let active = self.active_id == Some(id);

        self.dummy(size);
        self.last_item_hovered = hovered;
        self.last_item_active = active;
        pressed
    }

    /// `ImGui::Button`: a framed label. Returns true on the frame the mouse is released inside.
    pub fn button(&mut self, label: &str) -> bool {
        let id = self.get_id(label);
        let padding = self.style.frame_padding;
        let text = self.font.calc_text_size(label, self.font.size);
        let size = Vec2 {
            x: text.x + padding.x * 2.0,
            y: self.font.size + padding.y * 2.0,
        };
        let min = self.window().cursor;
        let pressed = self.interact(id, size);

        // Draw it: a filled frame in the state colour, then the label. FrameBorderSize is 0 in the
        // default style, so there is no border to draw.
        let max = Vec2 {
            x: min.x + size.x,
            y: min.y + size.y,
        };
        let frame_color = if self.last_item_active {
            self.style.color_u32(crate::style::Col::ButtonActive)
        } else if self.last_item_hovered {
            self.style.color_u32(crate::style::Col::ButtonHovered)
        } else {
            self.style.color_u32(crate::style::Col::Button)
        };
        self.draw_list.prim_rect(min, max, frame_color);
        let font = self.font.clone();
        let text_color = self.style.color_u32(crate::style::Col::Text);
        self.draw_list.add_text(
            &font,
            font.size,
            Vec2 {
                x: min.x + padding.x,
                y: min.y + padding.y,
            },
            text_color,
            label,
        );
        pressed
    }

    /// `ImGui::SliderFloat` (horizontal, not logarithmic): the value follows the mouse inside the
    /// usable range, with the reference's grab region and its 3-decimal rounding of the format.
    pub fn slider_float(
        &mut self,
        label: &str,
        value: &mut f32,
        min_value: f32,
        max_value: f32,
        width: f32,
    ) -> bool {
        const GRAB_PADDING: f32 = 2.0;
        const DECIMALS: f32 = 1000.0;

        let id = self.get_id(label);
        let frame_height = self.frame_height();
        let text = self.font.calc_text_size(label, self.font.size);
        let item_size = Vec2 {
            x: width + self.style.item_inner_spacing.x + text.x,
            y: frame_height,
        };

        let (min_pos, window_pos, window_size) = {
            let window = self.window();
            (window.cursor, window.pos, window.size)
        };
        let frame_max = Vec2 {
            x: min_pos.x + width,
            y: min_pos.y + frame_height,
        };

        let over_window = self
            .io
            .mouse_pos
            .map(|m| {
                m.x >= window_pos.x
                    && m.x < window_pos.x + window_size.x
                    && m.y >= window_pos.y
                    && m.y < window_pos.y + window_size.y
            })
            .unwrap_or(false);
        let hovered = over_window
            && self
                .io
                .mouse_pos
                .map(|m| {
                    m.x >= min_pos.x && m.x < frame_max.x && m.y >= min_pos.y && m.y < frame_max.y
                })
                .unwrap_or(false);

        if hovered && self.io.mouse_clicked[0] {
            self.active_id = Some(id);
            self.slider_just_activated = true;
        }

        let slider_sz = (frame_max.x - min_pos.x) - GRAB_PADDING * 2.0;
        let grab_sz = self.style.grab_min_size.min(slider_sz);
        let usable_sz = slider_sz - grab_sz;
        let usable_pos_min = min_pos.x + GRAB_PADDING + grab_sz * 0.5;
        let usable_pos_max = frame_max.x - GRAB_PADDING - grab_sz * 0.5;

        let was_active = self.active_id == Some(id);
        let mut changed = false;
        if was_active {
            if let Some(mouse) = self.io.mouse_pos {
                if self.slider_just_activated {
                    let ratio = if max_value > min_value {
                        (*value - min_value) / (max_value - min_value)
                    } else {
                        0.0
                    };
                    let grab_pos = usable_pos_min + (usable_pos_max - usable_pos_min) * ratio;
                    let around = mouse.x >= grab_pos - grab_sz * 0.5 - 1.0
                        && mouse.x <= grab_pos + grab_sz * 0.5 + 1.0;
                    self.slider_grab_offset = if around { mouse.x - grab_pos } else { 0.0 };
                    self.slider_just_activated = false;
                }
                if usable_sz > 0.0 {
                    let t = ((mouse.x - self.slider_grab_offset - usable_pos_min) / usable_sz)
                        .clamp(0.0, 1.0);
                    let raw = min_value + t * (max_value - min_value);
                    // `RoundScalarWithFormatT` with the "%.3f" default format.
                    let rounded = (raw * DECIMALS).round() / DECIMALS;
                    if *value != rounded {
                        *value = rounded;
                        changed = true;
                    }
                }
            }
        }

        if self.io.mouse_released[0] && was_active {
            self.active_id = None;
        }
        let active = self.active_id == Some(id);

        self.dummy(item_size);
        self.last_item_hovered = hovered;
        self.last_item_active = active;
        changed
    }

    /// `ImGui::Text` for an unformatted string: an item as wide as the text, no interaction.
    pub fn label(&mut self, text: &str) {
        let size = self.font.calc_text_size(text, self.font.size);
        self.dummy(size);
    }

    /// `ImGui::Checkbox`: a square box plus a label. Returns true on the frame it is toggled.
    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> bool {
        let id = self.get_id(label);
        let frame = self.frame_height();
        let text = self.font.calc_text_size(label, self.font.size);
        let size = Vec2 {
            x: frame + self.style.item_inner_spacing.x + text.x,
            y: frame,
        };
        let pressed = self.interact(id, size);
        if pressed {
            *value = !*value;
        }
        pressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_nest_like_the_reference() {
        let mut ctx = Context::new();
        assert_eq!(ctx.current_id(), 0);
        assert_eq!(ctx.get_id("Button"), hash_str_utf8("Button", 0));

        ctx.push_id("Window");
        let window_id = hash_str_utf8("Window", 0);
        assert_eq!(ctx.current_id(), window_id);
        assert_eq!(ctx.get_id("Button"), hash_str_utf8("Button", window_id));

        ctx.push_id("Child");
        let child_id = hash_str_utf8("Child", window_id);
        assert_eq!(ctx.current_id(), child_id);
        assert_eq!(ctx.get_id("Button"), hash_str_utf8("Button", child_id));

        ctx.pop_id();
        ctx.pop_id();
        assert_eq!(ctx.id_stack_depth(), 0);
        assert_eq!(ctx.current_id(), 0);
    }

    #[test]
    fn frame_counter_advances() {
        let mut ctx = Context::new();
        ctx.begin_frame();
        ctx.end_frame();
        ctx.begin_frame();
        assert_eq!(ctx.frame_count(), 2);
    }

    /// The numbers are the reference's, printed by `rust/tools/conformance` for `window_layout`.
    #[test]
    fn window_and_cursor_geometry_matches_reference() {
        let mut ctx = Context::new();
        ctx.begin_frame();

        ctx.begin_window(
            "W",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        assert_eq!(ctx.window().content_origin, Vec2 { x: 68.0, y: 68.0 });
        assert_eq!(ctx.content_avail(), Vec2 { x: 284.0, y: 184.0 });
        assert_eq!(ctx.cursor_screen_pos(), Vec2 { x: 68.0, y: 68.0 });

        ctx.dummy(Vec2 { x: 100.0, y: 20.0 });
        assert_eq!(ctx.item_rect_min(), Vec2 { x: 68.0, y: 68.0 });
        assert_eq!(ctx.item_rect_max(), Vec2 { x: 168.0, y: 88.0 });
        assert_eq!(ctx.cursor_screen_pos(), Vec2 { x: 68.0, y: 92.0 });

        ctx.same_line();
        assert_eq!(ctx.cursor_screen_pos(), Vec2 { x: 176.0, y: 68.0 });

        ctx.dummy(Vec2 { x: 50.0, y: 10.0 });
        assert_eq!(ctx.item_rect_max(), Vec2 { x: 226.0, y: 78.0 });
        assert_eq!(ctx.cursor_screen_pos(), Vec2 { x: 68.0, y: 92.0 });
        ctx.end_window();

        // Second frame without an explicit size: the window keeps what it had.
        ctx.begin_window("W", None, None);
        assert_eq!(ctx.window().pos, Vec2 { x: 60.0, y: 60.0 });
        assert_eq!(ctx.window().size, Vec2 { x: 300.0, y: 200.0 });
        ctx.end_window();
    }
}
