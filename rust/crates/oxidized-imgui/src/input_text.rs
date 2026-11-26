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
    /// `state->TextToRevertTo`: the buffer as it was when the field took the keyboard, which is what
    /// Escape goes back to.
    pub revert_to: String,
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
            revert_to: text.to_string(),
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

/// The cursor movements and edits the field models, named after the fake keys `stb_textedit` uses.
/// Shift, arrows, Home/End, Delete and Backspace are the whole catalogue for a single-line field.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EditKey {
    Left,
    Right,
    Home,
    End,
    Delete,
    Backspace,
}

impl InputTextState {
    /// `stb_textedit_clamp`: the cursor and the selection stay inside the text.
    pub fn cursor_clamp(&mut self) {
        let len = self.text.len();
        self.cursor = self.cursor.min(len);
        self.select_start = self.select_start.min(len);
        self.select_end = self.select_end.min(len);
    }

    /// `STB_TEXTEDIT_GETPREVCHARINDEX`: the previous character boundary, in bytes.
    fn prev_index(&self, index: usize) -> usize {
        let mut i = index.min(self.text.len());
        while i > 0 {
            i -= 1;
            if self.text.is_char_boundary(i) {
                return i;
            }
        }
        0
    }

    /// `STB_TEXTEDIT_GETNEXTCHARINDEX`: the next character boundary, in bytes.
    fn next_index(&self, index: usize) -> usize {
        let len = self.text.len();
        if index >= len {
            return len;
        }
        let mut i = index + 1;
        while i < len && !self.text.is_char_boundary(i) {
            i += 1;
        }
        i
    }

    /// `stb_textedit_delete`: take `len` bytes out at `where`.
    fn delete(&mut self, where_: usize, len: usize) {
        self.text.replace_range(where_..where_ + len, "");
    }

    /// `stb_textedit_delete_selection`: remove what is selected, leaving the caret where it was.
    pub fn delete_selection(&mut self) {
        self.cursor_clamp();
        if !self.has_selection() {
            return;
        }
        if self.select_start < self.select_end {
            self.delete(self.select_start, self.select_end - self.select_start);
            self.select_end = self.select_start;
            self.cursor = self.select_start;
        } else {
            self.delete(self.select_end, self.select_start - self.select_end);
            self.select_start = self.select_end;
            self.cursor = self.select_end;
        }
    }

    /// `stb_textedit_prep_selection_at_cursor`: an arrow with Shift starts a selection here.
    fn prep_selection_at_cursor(&mut self) {
        if self.has_selection() {
            self.cursor = self.select_end;
        } else {
            self.select_start = self.cursor;
            self.select_end = self.cursor;
        }
    }

    /// `stb_textedit_move_to_first`: collapse the selection to its start.
    fn move_to_first(&mut self) {
        if self.has_selection() {
            if self.select_end < self.select_start {
                core::mem::swap(&mut self.select_start, &mut self.select_end);
            }
            self.cursor = self.select_start;
            self.select_end = self.select_start;
        }
    }

    /// `stb_textedit_move_to_last`: collapse the selection to its end.
    fn move_to_last(&mut self) {
        if self.has_selection() {
            if self.select_end < self.select_start {
                core::mem::swap(&mut self.select_start, &mut self.select_end);
            }
            self.cursor_clamp();
            self.cursor = self.select_end;
            self.select_start = self.select_end;
        }
    }

    /// `ImGuiInputTextState::OnCharPressed`: insert a character at the caret, replacing whatever is
    /// selected.
    pub fn on_char_pressed(&mut self, c: char) {
        // A single-line field refuses the newline.
        if c == '\n' {
            return;
        }
        self.delete_selection();
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.cursor_follow = true;
        self.cursor_anim_reset();
    }

    /// `stb_textedit_cut`: take the selection out, leaving the caret where it was. With nothing
    /// selected the reference selects everything first.
    pub fn cut_selection(&mut self) {
        if !self.has_selection() {
            self.select_all();
        }
        self.delete_selection();
        // Only `OnKeyPressed`/`OnCharPressed` restart the blink; cut and paste just ask the view to
        // follow the caret.
        self.cursor_follow = true;
    }

    /// `stb_textedit_paste`: replace the selection with `text`.
    pub fn paste(&mut self, text: &str) {
        self.delete_selection();
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.cursor_follow = true;
    }

    /// `stb_textedit_replace`: the text becomes `text` and the caret and selection are clamped into
    /// it. Escape uses this to go back to what the field held when it was activated.
    pub fn replace_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor_clamp();
    }

    /// `ImGuiInputTextState::OnKeyPressed` for [`EditKey`]: the movement and the two deletions.
    ///
    /// `shift` is `STB_TEXTEDIT_K_SHIFT`: with it the movement drags the selection instead of
    /// collapsing it. Every key asks the view to follow the caret and restarts the blink.
    pub fn on_key(&mut self, key: EditKey, shift: bool) {
        match key {
            EditKey::Left => {
                if shift {
                    self.cursor_clamp();
                    self.prep_selection_at_cursor();
                    if self.select_end > 0 {
                        self.select_end = self.prev_index(self.select_end);
                    }
                    self.cursor = self.select_end;
                } else if self.has_selection() {
                    self.move_to_first();
                } else if self.cursor > 0 {
                    self.cursor = self.prev_index(self.cursor);
                }
            }
            EditKey::Right => {
                if shift {
                    self.prep_selection_at_cursor();
                    self.select_end = self.next_index(self.select_end);
                    self.cursor_clamp();
                    self.cursor = self.select_end;
                } else {
                    if self.has_selection() {
                        self.move_to_last();
                    } else {
                        self.cursor = self.next_index(self.cursor);
                    }
                    self.cursor_clamp();
                }
            }
            EditKey::Home => {
                // `STB_TEXTEDIT_K_LINESTART`: on a single line this is the start of the text.
                self.cursor_clamp();
                if shift {
                    self.prep_selection_at_cursor();
                    self.cursor = 0;
                    self.select_end = 0;
                } else {
                    self.move_to_first();
                    self.cursor = 0;
                }
            }
            EditKey::End => {
                // `STB_TEXTEDIT_K_LINEEND`: on a single line this is the end of the text.
                self.cursor_clamp();
                let len = self.text.len();
                if shift {
                    self.prep_selection_at_cursor();
                    self.cursor = len;
                    self.select_end = len;
                } else {
                    self.move_to_last();
                    self.cursor = len;
                }
            }
            EditKey::Backspace => {
                if self.has_selection() {
                    self.delete_selection();
                } else {
                    self.cursor_clamp();
                    if self.cursor > 0 {
                        let prev = self.prev_index(self.cursor);
                        self.delete(prev, self.cursor - prev);
                        self.cursor = prev;
                    }
                }
            }
            EditKey::Delete => {
                if self.has_selection() {
                    self.delete_selection();
                } else if self.cursor < self.text.len() {
                    let next = self.next_index(self.cursor);
                    self.delete(self.cursor, next - self.cursor);
                }
            }
        }
        self.cursor_follow = true;
        self.cursor_anim_reset();
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

    #[test]
    fn typing_replaces_the_selection_and_moves_the_caret() {
        let mut state = InputTextState::new(1, "Hello");
        state.cursor = 5;
        state.select_start = 1;
        state.select_end = 3;
        state.on_char_pressed('Z');
        assert_eq!(state.text, "HZlo");
        assert_eq!(state.cursor, 2);
        assert!(!state.has_selection());
        assert!(state.cursor_follow);
    }

    #[test]
    fn backspace_and_delete_remove_one_character() {
        let mut state = InputTextState::new(1, "abc");
        state.cursor = 2;
        state.on_key(EditKey::Backspace, false);
        assert_eq!(state.text, "ac");
        assert_eq!(state.cursor, 1);
        state.on_key(EditKey::Delete, false);
        assert_eq!(state.text, "a");
        // Nothing left in front of the caret.
        state.on_key(EditKey::Delete, false);
        assert_eq!(state.text, "a");
        state.on_key(EditKey::End, false);
        assert_eq!(state.cursor, 1);
        state.on_key(EditKey::Home, false);
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn shift_drags_the_selection_and_a_plain_move_collapses_it() {
        let mut state = InputTextState::new(1, "abcd");
        state.cursor = 4;
        state.on_key(EditKey::Home, false);
        state.on_key(EditKey::Right, true);
        state.on_key(EditKey::Right, true);
        assert_eq!((state.select_start, state.select_end), (0, 2));
        assert_eq!(state.cursor, 2);
        assert!(state.has_selection());
        // A plain move collapses the selection to its start.
        state.on_key(EditKey::Left, false);
        assert_eq!(state.cursor, 0);
        assert!(!state.has_selection());
    }

    #[test]
    fn cut_takes_the_selection_or_everything() {
        let mut state = InputTextState::new(1, "abcdef");
        state.cursor = 3;
        state.select_start = 1;
        state.select_end = 4;
        state.cut_selection();
        assert_eq!(state.text, "aef");
        assert_eq!(state.cursor, 1);
        // With nothing selected the reference takes the whole text.
        state.select_start = state.cursor;
        state.select_end = state.cursor;
        state.cut_selection();
        assert_eq!(state.text, "");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn paste_replaces_the_selection_and_escape_restores_the_activation_value() {
        let mut state = InputTextState::new(1, "abc");
        state.cursor = 3;
        state.select_start = 1;
        state.select_end = 3;
        state.paste("XY");
        assert_eq!(state.text, "aXY");
        assert_eq!(state.cursor, 3);

        // Escape: `stb_textedit_replace` with what the field held when it was activated.
        let revert = state.revert_to.clone();
        state.replace_text(&revert);
        assert_eq!(state.text, "abc");
        state.cursor_clamp();
        assert!(state.cursor <= 3);
    }

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
