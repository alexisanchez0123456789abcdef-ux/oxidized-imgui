//! `ImGuiInputTextState`: the edit state of the field that owns the keyboard.
//!
//! The reference keeps a single state in the context (`g.InputTextState`), taken over when a field
//! is activated and released when it loses the focus. While the field is active this copy is what
//! gets displayed and edited; the caller's buffer is written back when the edit ends.
//!
//! The cursor and the selection are byte indices into `text`, as in the reference (which stores
//! UTF-8 byte offsets, not character counts).

use crate::draw_list::Vec2;
use crate::font::Font;
use crate::id::ImGuiID;

/// What a field remembers while it is being edited. Single line: there is no line index.
#[derive(Clone, Debug, PartialEq)]
pub struct InputTextState {
    /// `state->ID`: the field this state belongs to.
    pub id: ImGuiID,
    /// `state->TextA`: the working copy, of the same length the caller's buffer had.
    pub text: String,
    /// `Stb->cursor`: where the caret sits, in bytes.
    pub cursor: usize,
    /// `Stb->select_start` / `Stb->select_end`: the selection, in bytes, either order.
    pub select_start: usize,
    pub select_end: usize,
    /// `state->Scroll.x`: the horizontal offset of the text inside the field. Only the keyboard
    /// moves it, which is why a click can put the caret outside the visible part and leave it there.
    pub scroll_x: f32,
    /// `state->CursorAnim`: the blink timer. Negative right after an input, so the caret stays on.
    pub cursor_anim: f32,
    /// `state->CursorFollow`: scroll to the caret on this frame.
    pub cursor_follow: bool,
}

impl InputTextState {
    /// A state for a field that has just been activated, holding a copy of the caller's buffer.
    pub fn new(id: ImGuiID, text: &str) -> Self {
        Self {
            id,
            text: text.to_string(),
            cursor: 0,
            select_start: 0,
            select_end: 0,
            scroll_x: 0.0,
            cursor_anim: 0.0,
            cursor_follow: false,
        }
    }

    /// `HasSelection`.
    pub fn has_selection(&self) -> bool {
        self.select_start != self.select_end
    }

    /// `CursorAnimReset`: `-0.30`, so the caret stays on for a few frames after an input.
    pub fn cursor_anim_reset(&mut self) {
        self.cursor_anim = -0.30;
    }

    /// `CursorClamp`: a stale state cannot point past the text.
    pub fn cursor_clamp(&mut self) {
        let len = self.text.len();
        self.cursor = self.cursor.min(len);
        self.select_start = self.select_start.min(len);
        self.select_end = self.select_end.min(len);
    }

    /// `SelectAll`.
    pub fn select_all(&mut self) {
        self.select_start = 0;
        self.cursor = self.text.len();
        self.select_end = self.text.len();
    }

    /// `GetCursorPos`.
    pub fn cursor_pos(&self) -> usize {
        self.cursor
    }

    /// `state->Scroll`, as a point: only the horizontal part is modelled.
    pub fn scroll(&self) -> Vec2 {
        Vec2 {
            x: self.scroll_x,
            y: 0.0,
        }
    }

    /// `stb_text_locate_coord` for a single line: which byte index the pixel `x` falls on.
    ///
    /// The reference walks the characters of the row and rounds to the nearest boundary, so a click
    /// past the middle of a glyph lands after it. Past the end of the line it answers "after the
    /// last character".
    pub fn locate_coord(&self, x: f32, font: &Font, size: f32) -> usize {
        locate_coord_in(&self.text, x, font, size)
    }
}

/// The byte index the pixel `x` falls on inside `text`, as `stb_text_locate_coord` computes it for
/// a single line: `x` is measured from the start of the text, in pixels.
pub fn locate_coord_in(text: &str, x: f32, font: &Font, size: f32) -> usize {
    // `r.x0 = 0` and `r.x1 = size.x`, rounded up to a whole pixel by `InputTextCalcTextSize`.
    let line_width = (font.calc_text_size(text, size).x + 0.999_99).trunc();
    if x < 0.0 {
        return 0;
    }
    if x < line_width {
        let mut prev_x = 0.0f32;
        for (index, ch) in text.char_indices() {
            let width = font.advance_of(ch, size);
            if x < prev_x + width {
                if x < prev_x + width * 0.5 {
                    return index;
                }
                return index + ch.len_utf8();
            }
            prev_x += width;
        }
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default font is monospaced at seven pixels, which is what makes these numbers readable:
    /// "Hello world" is eleven characters and seventy-seven pixels wide.
    fn font() -> Font {
        Font::reference_default()
    }

    #[test]
    fn a_click_rounds_to_the_nearest_character_boundary() {
        let font = font();
        let text = "Hello world";
        // Before the line: the first character.
        assert_eq!(locate_coord_in(text, -3.0, &font, 13.0), 0);
        assert_eq!(locate_coord_in(text, 0.0, &font, 13.0), 0);
        // Inside the first glyph, before its middle: still the first character.
        assert_eq!(locate_coord_in(text, 3.0, &font, 13.0), 0);
        // Past its middle: the second.
        assert_eq!(locate_coord_in(text, 4.0, &font, 13.0), 1);
        // Exactly on a boundary: that boundary.
        assert_eq!(locate_coord_in(text, 28.0, &font, 13.0), 4);
        // Past the end of the line: after the last character.
        assert_eq!(locate_coord_in(text, 76.9, &font, 13.0), 11);
        assert_eq!(locate_coord_in(text, 200.0, &font, 13.0), 11);
    }

    #[test]
    fn a_state_starts_empty_and_holds_a_copy() {
        let state = InputTextState::new(7, "abc");
        assert_eq!(state.id, 7);
        assert_eq!(state.text, "abc");
        assert_eq!(state.cursor, 0);
        assert!(!state.has_selection());
    }

    #[test]
    fn clamping_and_selecting_all_follow_the_text_length() {
        let mut state = InputTextState::new(1, "hello");
        state.cursor = 99;
        state.select_start = 99;
        state.select_end = 42;
        state.cursor_clamp();
        assert_eq!(state.cursor, 5);
        assert_eq!(state.select_start, 5);
        assert_eq!(state.select_end, 5);

        state.select_all();
        assert_eq!(state.cursor, 5);
        assert_eq!(state.select_start, 0);
        assert_eq!(state.select_end, 5);
        assert!(state.has_selection());
    }

    #[test]
    fn the_blink_timer_resets_negative() {
        let mut state = InputTextState::new(1, "");
        state.cursor_anim = 5.0;
        state.cursor_anim_reset();
        assert_eq!(state.cursor_anim, -0.30);
    }
}
