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
    /// `undostate`: the edits that can be undone, oldest first.
    undo: Vec<UndoRecord>,
    /// `undostate`: the edits that can be redone.
    redo: Vec<UndoRecord>,
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
            undo: Vec::new(),
            redo: Vec::new(),
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

/// `IMSTB_TEXTEDIT_UNDOSTATECOUNT`: how many edits the undo stack remembers.
const UNDO_COUNT: usize = 99;

/// One entry of the undo stack, seen from the undo's point of view: remove `delete_length` bytes at
/// `at`, then put `reinsert` back. The reference keeps the same record in a fixed array plus a
/// shared character buffer; one `Vec` per record is the same model without the bookkeeping.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UndoRecord {
    at: usize,
    delete_length: usize,
    reinsert: Vec<u8>,
}

/// `ImCharIsBlankW`.
fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\u{3000}'
}

/// `ImCharIsSeparatorW`, the reference's table verbatim.
fn is_separator(c: char) -> bool {
    matches!(
        c,
        ',' | '\u{3001}'
            | '.'
            | '\u{3002}'
            | ';'
            | '\u{FF1B}'
            | '('
            | '\u{FF08}'
            | ')'
            | '\u{FF09}'
            | '{'
            | '\u{FF5B}'
            | '}'
            | '\u{FF5D}'
            | '['
            | '\u{300C}'
            | ']'
            | '\u{300D}'
            | '|'
            | '\u{FF5C}'
            | '!'
            | '\u{FF01}'
            | '\\'
            | '\u{FFE5}'
            | '/'
            | '\u{30FB}'
            | '\u{FF0F}'
            | '\n'
            | '\r'
    )
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

    /// `stb_textedit_delete`: the same, remembering the edit so it can be undone.
    fn delete_and_record(&mut self, where_: usize, len: usize) {
        self.make_undo_delete(where_, len);
        self.delete(where_, len);
    }

    /// `stb_textedit_clear_state`: a new keyboard session starts with empty stacks.
    pub fn clear_undo(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// `stb_textedit_flush_redo`: a fresh edit drops the redo branch.
    fn flush_redo(&mut self) {
        self.redo.clear();
    }

    /// `stb_text_createundo`: push a record, keeping the stack bounded like the reference's array.
    fn push_undo(&mut self, record: UndoRecord) {
        self.flush_redo();
        if self.undo.len() == UNDO_COUNT {
            self.undo.remove(0);
        }
        self.undo.push(record);
    }

    /// `stb_text_makeundo_insert`: the undo has to remove what the edit inserted.
    fn make_undo_insert(&mut self, at: usize, length: usize) {
        self.push_undo(UndoRecord {
            at,
            delete_length: length,
            reinsert: Vec::new(),
        });
    }

    /// `stb_text_makeundo_delete`: the undo has to put back what the edit removed.
    fn make_undo_delete(&mut self, at: usize, length: usize) {
        let reinsert = self.text.as_bytes()[at..at + length].to_vec();
        self.push_undo(UndoRecord {
            at,
            delete_length: 0,
            reinsert,
        });
    }

    /// `stb_text_undo`. True when there was something to undo.
    pub fn undo(&mut self) -> bool {
        let Some(record) = self.undo.pop() else {
            return false;
        };
        let inverse = self.apply_record(record);
        self.redo.push(inverse);
        true
    }

    /// `stb_text_redo`. True when there was something to redo.
    pub fn redo(&mut self) -> bool {
        let Some(record) = self.redo.pop() else {
            return false;
        };
        let inverse = self.apply_record(record);
        self.undo.push(inverse);
        true
    }

    /// Apply one record to the text and return its inverse, which is what the other stack needs.
    fn apply_record(&mut self, record: UndoRecord) -> UndoRecord {
        let at = record.at.min(self.text.len());
        let remove = record.delete_length.min(self.text.len() - at);
        let removed = self.text.as_bytes()[at..at + remove].to_vec();
        self.text.replace_range(at..at + remove, "");
        // The record only ever holds whole UTF-8 sequences.
        let reinsert = String::from_utf8_lossy(&record.reinsert).into_owned();
        self.text.insert_str(at, &reinsert);
        self.cursor = at + record.reinsert.len();
        while self.cursor > 0 && !self.text.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }
        self.select_start = self.cursor;
        self.select_end = self.cursor;
        self.cursor_follow = true;
        self.cursor_anim_reset();
        UndoRecord {
            at,
            delete_length: record.reinsert.len(),
            reinsert: removed,
        }
    }

    /// The character at `index`, if there is one.
    fn char_at(&self, index: usize) -> Option<char> {
        self.text.get(index..).and_then(|rest| rest.chars().next())
    }

    /// The character before `index`, if there is one.
    fn char_before(&self, index: usize) -> Option<char> {
        self.text
            .get(..index)
            .and_then(|head| head.chars().next_back())
    }

    /// `is_word_boundary_from_right`: a word starts here when walking backwards.
    fn is_word_boundary_from_right(&self, index: usize) -> bool {
        if index == 0 {
            return false;
        }
        let (Some(curr), Some(prev)) = (self.char_at(index), self.char_before(index)) else {
            return false;
        };
        let (prev_white, prev_separ) = (is_blank(prev), is_separator(prev));
        let (curr_white, curr_separ) = (is_blank(curr), is_separator(curr));
        ((prev_white || prev_separ) && !(curr_separ || curr_white)) || (curr_separ && !prev_separ)
    }

    /// `is_word_boundary_from_left`. The reference reads the two characters with their names swapped
    /// and compares them that way, which is reproduced here as written.
    fn is_word_boundary_from_left(&self, index: usize) -> bool {
        if index == 0 {
            return false;
        }
        let (Some(at), Some(before)) = (self.char_at(index), self.char_before(index)) else {
            return false;
        };
        // `prev_c` in the reference is the character at `idx`, `curr_c` the one before it.
        let (prev_white, prev_separ) = (is_blank(at), is_separator(at));
        let (curr_white, curr_separ) = (is_blank(before), is_separator(before));
        (prev_white && !(curr_separ || curr_white)) || (curr_separ && !prev_separ)
    }

    /// `STB_TEXTEDIT_MOVEWORDLEFT_IMPL`.
    fn move_word_left(&self, index: usize) -> usize {
        let mut i = self.prev_index(index);
        while i > 0 && !self.is_word_boundary_from_right(i) {
            i = self.prev_index(i);
        }
        i
    }

    /// `STB_TEXTEDIT_MOVEWORDRIGHT_MAC`, the one the reference uses for double-click.
    fn move_word_right_mac(&self, index: usize) -> usize {
        let len = self.text.len();
        let mut i = self.next_index(index);
        while i < len && !self.is_word_boundary_from_left(i) {
            let next = self.next_index(i);
            if next == i {
                break;
            }
            i = next;
        }
        i.min(len)
    }

    /// `stb_textedit_click`: the caret lands where the mouse is and the selection collapses.
    pub fn click(&mut self, x: f32, font: &Font, size: f32) {
        let p = self.locate_coord(x, font, size);
        self.cursor = p;
        self.select_start = p;
        self.select_end = p;
    }

    /// `stb_textedit_drag`: the drag keeps the anchor and moves the other end.
    pub fn drag(&mut self, x: f32, font: &Font, size: f32) {
        if !self.has_selection() {
            self.select_start = self.cursor;
        }
        let p = self.locate_coord(x, font, size);
        self.cursor = p;
        self.select_end = p;
    }

    /// A double click selects the word under the mouse: the reference clicks, walks to the word
    /// start and then to the word end with the "Mac" advance.
    pub fn double_click(&mut self, x: f32, font: &Font, size: f32) {
        self.click(x, font, size);
        let at_bol = self.cursor == 0 || self.char_before(self.cursor) == Some('\n');
        if self.has_selection() || !at_bol {
            // `OnKeyPressed(STB_TEXTEDIT_K_WORDLEFT)`: collapse a selection or walk back one word.
            if self.has_selection() {
                self.move_to_first_anchored();
            } else {
                self.cursor = self.move_word_left(self.cursor);
                self.cursor_clamp();
            }
        }
        if !self.has_selection() {
            self.select_start = self.cursor;
            self.select_end = self.cursor;
        }
        self.cursor = self.move_word_right_mac(self.cursor);
        self.select_end = self.cursor;
        self.cursor_clamp();
        self.cursor_follow = true;
        self.cursor_anim_reset();
    }

    /// The collapse half of `stb_textedit_move_to_first`, for the double-click path.
    fn move_to_first_anchored(&mut self) {
        if self.select_end < self.select_start {
            core::mem::swap(&mut self.select_start, &mut self.select_end);
        }
        self.cursor = self.select_start;
        self.select_end = self.select_start;
    }

    /// `stb_textedit_delete_selection`: remove what is selected, leaving the caret where it was.
    pub fn delete_selection(&mut self) {
        self.cursor_clamp();
        if !self.has_selection() {
            return;
        }
        if self.select_start < self.select_end {
            self.delete_and_record(self.select_start, self.select_end - self.select_start);
            self.select_end = self.select_start;
            self.cursor = self.select_start;
        } else {
            self.delete_and_record(self.select_end, self.select_start - self.select_end);
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
        self.make_undo_insert(self.cursor, c.len_utf8());
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
        self.make_undo_insert(self.cursor, text.len());
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
                        self.delete_and_record(prev, self.cursor - prev);
                        self.cursor = prev;
                    }
                }
            }
            EditKey::Delete => {
                if self.has_selection() {
                    self.delete_selection();
                } else if self.cursor < self.text.len() {
                    let next = self.next_index(self.cursor);
                    self.delete_and_record(self.cursor, next - self.cursor);
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
