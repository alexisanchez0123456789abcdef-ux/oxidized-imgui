//! `ImGuiMenuColumns`: the column measurement menu items share inside one window.
//!
//! A vertical menu lays its rows out in four columns — icon, label, shortcut and a reserved mark
//! column — so that the label of every row starts at the same x and the shortcuts line up on the
//! right. The widths are accumulated while a frame is submitted and then *locked*: during a frame
//! the offsets are still the previous frame's, which is why a menu settles on its second frame.
//!
//! The algorithm is the reference's (`imgui_widgets.cpp`), integer types included: widths, offsets
//! and spacing are truncated to `u16`, so a fractional advance is dropped exactly where the C++
//! drops it.

/// One window's menu columns, kept between frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuColumns {
    /// The width the columns took on the previous frame, which is what a menu item is laid out with.
    total_width: u32,
    /// The width accumulated so far during this frame.
    next_total_width: u32,
    /// `ItemSpacing.x`, truncated.
    spacing: u16,
    /// `OffsetIcon`: always zero for now, as in the reference.
    offset_icon: u16,
    /// Where the label column starts, locked by [`MenuColumns::update`].
    offset_label: u16,
    /// Where the shortcut column starts.
    offset_shortcut: u16,
    /// Where the mark column starts.
    offset_mark: u16,
    /// The widest entry of each column so far this frame: icon, label, shortcut, mark.
    widths: [u16; 4],
}

impl MenuColumns {
    /// `ImGuiMenuColumns::Update`: lock the offsets with the widths of the previous frame, then
    /// start the new frame from zero.
    ///
    /// `window_reappearing` is the reference's `window_just_activated_by_user`: a window that was
    /// not submitted on the previous frame throws its measurements away instead of locking them,
    /// which is what makes a menu that has just been opened lay its first row out against zero.
    pub fn update(&mut self, spacing: f32, window_reappearing: bool) {
        if window_reappearing {
            self.widths = [0; 4];
        }
        self.spacing = spacing as u16;
        self.calc_next_total_width(true);
        self.widths = [0; 4];
        self.total_width = self.next_total_width;
        self.next_total_width = 0;
    }

    /// `ImGuiMenuColumns::CalcNextTotalWidth`: walk the four columns and total them, adding the
    /// spacing only *between* the columns that exist, since a zero-width column is skipped.
    fn calc_next_total_width(&mut self, update_offsets: bool) {
        let mut offset: u16 = 0;
        let mut want_spacing = false;
        for index in 0..self.widths.len() {
            let width = self.widths[index];
            if want_spacing && width > 0 {
                offset += self.spacing;
            }
            want_spacing |= width > 0;
            if update_offsets {
                if index == 1 {
                    self.offset_label = offset;
                }
                if index == 2 {
                    self.offset_shortcut = offset;
                }
                if index == 3 {
                    self.offset_mark = offset;
                }
            }
            offset += width;
        }
        self.next_total_width = u32::from(offset);
    }

    /// `ImGuiMenuColumns::DeclColumns`: fold one row's column widths into the frame's totals and
    /// answer the width the row should be laid out with, which is the wider of the previous frame's
    /// locked total and the one being accumulated.
    pub fn decl_columns(&mut self, w_icon: f32, w_label: f32, w_shortcut: f32, w_mark: f32) -> f32 {
        self.widths[0] = self.widths[0].max(w_icon as u16);
        self.widths[1] = self.widths[1].max(w_label as u16);
        self.widths[2] = self.widths[2].max(w_shortcut as u16);
        self.widths[3] = self.widths[3].max(w_mark as u16);
        self.calc_next_total_width(false);
        self.total_width.max(self.next_total_width) as f32
    }

    /// `OffsetIcon`: where the icon column starts. Always zero for now, as in the reference.
    pub fn offset_icon(&self) -> f32 {
        f32::from(self.offset_icon)
    }

    /// `OffsetLabel`: where the label column starts.
    pub fn offset_label(&self) -> f32 {
        f32::from(self.offset_label)
    }

    /// `OffsetShortcut`: where the shortcut column starts.
    pub fn offset_shortcut(&self) -> f32 {
        f32::from(self.offset_shortcut)
    }

    /// `OffsetMark`: where the reserved mark column starts.
    pub fn offset_mark(&self) -> f32 {
        f32::from(self.offset_mark)
    }

    /// The width the columns took on the previous frame.
    pub fn total_width(&self) -> f32 {
        self.total_width as f32
    }

    /// The width accumulated so far this frame.
    pub fn next_total_width(&self) -> f32 {
        self.next_total_width as f32
    }
}

/// The width a menu reserves for the mark column: `IM_TRUNC(FontSize * 1.20f)`, always accounted
/// for even in a menu with no selectable row, because `selected` has no neutral setting.
pub fn checkmark_width(font_size: f32) -> f32 {
    (font_size * 1.20).trunc()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers are the reference's: the `submenu` scenario of the conformance driver measures a
    /// menu whose label column is 28 wide and whose shortcut column is 42 (`New` / `Ctrl+N` in the
    /// default font, seven pixels per advance), with a 15 pixel mark column.
    #[test]
    fn columns_settle_on_the_second_frame() {
        // First frame: nothing is locked yet, so each row is laid out with what has accumulated.
        let mut columns = MenuColumns::default();
        columns.update(8.0, true);
        assert_eq!(columns.decl_columns(0.0, 21.0, 42.0, 15.0), 94.0); // New + Ctrl+N
        assert_eq!(columns.decl_columns(0.0, 28.0, 0.0, 15.0), 101.0); // Open widens the label

        // Second frame: the label column is now wide enough for "Open", and the offsets are locked
        // from the first frame's widths.
        columns.update(8.0, false);
        assert_eq!(columns.offset_icon(), 0.0);
        assert_eq!(columns.offset_label(), 0.0);
        assert_eq!(columns.offset_shortcut(), 36.0); // 28 label + 8 spacing
        assert_eq!(columns.offset_mark(), 86.0); // + 42 shortcut + 8 spacing
        assert_eq!(columns.total_width(), 101.0);
        assert_eq!(columns.decl_columns(0.0, 21.0, 42.0, 15.0), 101.0);
        assert_eq!(columns.decl_columns(0.0, 28.0, 0.0, 15.0), 101.0);
    }

    #[test]
    fn spacing_only_between_columns_that_exist() {
        // A menu with no shortcut: the mark column follows the label after one spacing.
        let mut columns = MenuColumns::default();
        columns.update(8.0, true);
        columns.decl_columns(0.0, 28.0, 0.0, 15.0);
        columns.update(8.0, false);
        assert_eq!(columns.offset_label(), 0.0);
        assert_eq!(columns.offset_shortcut(), 28.0); // the empty shortcut column starts right after
        assert_eq!(columns.offset_mark(), 36.0); // 28 + 8 spacing, the empty column adds no width
        assert_eq!(columns.total_width(), 51.0); // 28 + 8 + 15
    }

    #[test]
    fn an_icon_shifts_every_column_that_follows() {
        let mut columns = MenuColumns::default();
        columns.update(8.0, true);
        columns.decl_columns(7.0, 28.0, 42.0, 15.0);
        columns.update(8.0, false);
        assert_eq!(columns.offset_label(), 15.0); // 7 icon + 8 spacing
        assert_eq!(columns.offset_shortcut(), 51.0);
        assert_eq!(columns.offset_mark(), 101.0);
        assert_eq!(columns.total_width(), 116.0);
    }

    #[test]
    fn a_reappearing_window_forgets_the_previous_frame() {
        let mut columns = MenuColumns::default();
        columns.update(8.0, true);
        columns.decl_columns(0.0, 28.0, 42.0, 15.0);
        columns.update(8.0, false);
        assert_eq!(columns.total_width(), 101.0);
        // The menu was closed and re-opened: the widths are thrown away, not locked.
        columns.update(8.0, true);
        assert_eq!(columns.total_width(), 0.0);
        assert_eq!(columns.offset_shortcut(), 0.0);
    }

    #[test]
    fn rounding_is_truncation_like_the_reference() {
        // A fractional advance is truncated to u16 before it is totalled, exactly as the C++ casts.
        let mut columns = MenuColumns::default();
        columns.update(7.5, true);
        columns.decl_columns(0.0, 20.5, 0.0, 15.9);
        columns.update(7.5, false);
        assert_eq!(columns.offset_shortcut(), 20.0); // 20 label, no spacing after an empty column
        assert_eq!(columns.offset_mark(), 27.0); // 20 + 7 spacing
        assert_eq!(columns.total_width(), 42.0); // 20 + 7 + 15
    }

    #[test]
    fn checkmark_column_is_a_fraction_of_the_font() {
        assert_eq!(checkmark_width(13.0), 15.0); // trunc(15.6)
        assert_eq!(checkmark_width(16.0), 19.0); // trunc(19.2)
    }
}
