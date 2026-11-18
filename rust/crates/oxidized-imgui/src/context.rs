//! Context, windows and cursor layout.
//!
//! The part of the reference that decides where things go: a window with an explicit position and
//! size, its content region, and a cursor advancing over items (`Dummy`, `SameLine`). Text
//! metrics, widgets, scrolling, stacking and settings are not implemented yet.
//!
//! Numbers here are verified against the reference by the `window_layout` conformance scenario.

use crate::draw_list::Vec2;
use crate::id::{ImGuiID, hash_str_utf8};
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

/// Frame state, identifier stack, style and the open window stack.
#[derive(Debug, Default)]
pub struct Context {
    id_stack: Vec<ImGuiID>,
    frame_count: u64,
    last_frame_draw_calls: usize,
    style: Style,
    windows: Vec<Window>,
    remembered: Vec<(String, Vec2, Vec2)>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            style: Style::default(),
            ..Self::default()
        }
    }

    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Starts a frame. The reference also validates the IO and resets the draw data.
    pub fn begin_frame(&mut self) {
        self.frame_count += 1;
    }

    /// Ends a frame.
    pub fn end_frame(&mut self) {}

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
    pub fn begin_window(&mut self, name: &str, pos: Option<Vec2>, size: Option<Vec2>) {
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
