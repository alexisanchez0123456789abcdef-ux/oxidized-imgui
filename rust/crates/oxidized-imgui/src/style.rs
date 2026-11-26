//! Style: colours, roundings and border sizes.
//!
//! This is the *value* layer of appearance: what colour a widget samples in each state and how
//! round its corners are. The *geometry* of the anti-aliased fills (the fringe/miter/bevel
//! pipeline) is a separate, much larger piece and is not here yet.

use crate::draw_list::Vec2;

/// `ImGuiCol_` subset used by the widgets this crate implements.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Col {
    Text,
    TextDisabled,
    WindowBg,
    FrameBg,
    FrameBgHovered,
    FrameBgActive,
    Button,
    ButtonHovered,
    ButtonActive,
    CheckMark,
    SliderGrab,
    SliderGrabActive,
    Border,
    Separator,
    Header,
    TitleBg,
    TitleBgActive,
    TitleBgCollapsed,
    ScrollbarBg,
    ScrollbarGrab,
    ScrollbarGrabHovered,
    ScrollbarGrabActive,
    ResizeGrip,
    ResizeGripHovered,
    ResizeGripActive,
    SeparatorHovered,
    SeparatorActive,
    HeaderHovered,
    HeaderActive,
    /// `ImGuiCol_TextSelectedBg`: the background of a text selection.
    TextSelectedBg,
    /// `ImGuiCol_InputTextCursor`: the caret of a text field.
    InputTextCursor,
}

impl Col {
    pub const COUNT: usize = 31;

    pub(crate) fn index(self) -> usize {
        match self {
            Col::Text => 0,
            Col::TextDisabled => 1,
            Col::WindowBg => 2,
            Col::FrameBg => 3,
            Col::FrameBgHovered => 4,
            Col::FrameBgActive => 5,
            Col::Button => 6,
            Col::ButtonHovered => 7,
            Col::ButtonActive => 8,
            Col::CheckMark => 9,
            Col::SliderGrab => 10,
            Col::SliderGrabActive => 11,
            Col::Border => 12,
            Col::Separator => 13,
            Col::Header => 14,
            Col::HeaderHovered => 15,
            Col::HeaderActive => 16,
            Col::TitleBg => 17,
            Col::TitleBgActive => 18,
            Col::TitleBgCollapsed => 19,
            Col::ScrollbarBg => 20,
            Col::ScrollbarGrab => 21,
            Col::ScrollbarGrabHovered => 22,
            Col::ScrollbarGrabActive => 23,
            Col::ResizeGrip => 24,
            Col::ResizeGripHovered => 25,
            Col::ResizeGripActive => 26,
            Col::SeparatorHovered => 27,
            Col::SeparatorActive => 28,
            Col::TextSelectedBg => 29,
            Col::InputTextCursor => 30,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Style {
    pub window_padding: Vec2,
    pub item_spacing: Vec2,
    pub frame_padding: Vec2,
    pub item_inner_spacing: Vec2,
    /// `Style::CellPadding`: the padding of a table cell. The default is `(4, 2)`, which is **not**
    /// `FramePadding`: a row holding 13 px of text is 17 px tall because of it.
    pub cell_padding: Vec2,
    /// `Style::GrabMinSize`: the grab of a slider.
    pub grab_min_size: f32,
    /// `Style::IndentSpacing`: how far a tree node's children move right.
    pub indent_spacing: f32,
    /// `Style::SelectableRounding`: the rounding of a selectable row's highlight.
    pub selectable_rounding: f32,
    /// `ImGuiCol_PopupBg`: what a popup draws instead of the window background.
    pub popup_bg: u32,
    /// `ImGuiCol_MenuBarBg`: the band behind a menu bar.
    pub menu_bar_bg: u32,
    /// `ImGuiCol_TableHeaderBg`: the band behind a table's header row.
    pub table_header_bg: u32,
    /// `ImGuiCol_HeaderHovered`: the fill of a header cell under the mouse.
    pub header_hovered: u32,
    /// `ImGuiCol_HeaderActive`: the fill of a header cell held with the mouse.
    pub header_active: u32,
    /// `ImGuiCol_TableRowBg`: the even rows of a `RowBg` table. Fully transparent by default.
    pub table_row_bg: u32,
    /// `ImGuiCol_TableRowBgAlt`: the odd rows of a `RowBg` table, white at 6 % alpha.
    pub table_row_bg_alt: u32,
    /// `ImGuiCol_TableBorderStrong`: the outer border and the line under a header row.
    pub table_border_strong: u32,
    /// `ImGuiCol_TableBorderLight`: the borders between data rows and between columns.
    pub table_border_light: u32,
    /// `Style::WindowMinSize`: the smallest size a requested window size is clamped to.
    pub window_min_size: Vec2,
    /// `Style::ScrollbarSize`: the width of a scrollbar, taken out of the window's interior.
    pub scrollbar_size: f32,
    /// `Style::SeparatorSize`: the thickness of a `Separator`, at least one pixel.
    pub separator_size: f32,
    /// `Style::SeparatorTextPadding`: the gap between the label and the lines of `SeparatorText`.
    pub separator_text_padding: Vec2,
    /// `Style::SeparatorTextBorderSize`: the thickness of those lines.
    pub separator_text_border_size: f32,
    /// `Style::SeparatorTextAlign`: where the label sits inside the item.
    pub separator_text_align: Vec2,
    /// `Style::Alpha`: the global alpha every packed colour is multiplied by, 1 by default.
    pub alpha: f32,
    /// `Style::DisabledAlpha`: the factor `BeginDisabled` applies to `Style::Alpha`, 0.6 as in the
    /// reference.
    pub disabled_alpha: f32,
    /// `Style::ScrollbarPadding`: the gap between the scrollbar frame and its grab.
    pub scrollbar_padding: f32,

    pub frame_rounding: f32,
    pub window_rounding: f32,
    pub grab_rounding: f32,
    pub child_rounding: f32,
    pub popup_rounding: f32,
    pub scrollbar_rounding: f32,
    pub tab_rounding: f32,

    pub window_border_size: f32,
    /// Style::WindowBorderHoverPadding: how far outside a border counts as its handle.
    pub window_border_hover_padding: f32,
    pub frame_border_size: f32,
    pub child_border_size: f32,
    pub popup_border_size: f32,

    /// `Style::SymbolArrowSizeFactor`: width and height of an arrow, as a fraction of its radius.
    pub symbol_arrow_size_factor: Vec2,
    /// `Style::SymbolArrowThicknessFactor`: stroke weight of an arrow relative to its radius.
    /// `0.0` makes the reference fill the arrow instead of stroking it.
    pub symbol_arrow_thickness_factor: f32,
    /// `Style::DisplayWindowPadding` and `Style::DisplaySafeAreaPadding`: together they decide how
    /// much of a window may leave the work area when it is moved.
    pub display_window_padding: Vec2,
    pub display_safe_area_padding: Vec2,
    /// `Style::WindowTitleAlign`: where the title sits inside its layout rectangle. Only the
    /// default `(0.0, 0.5)` — flush left, vertically centred — is modelled.
    pub window_title_align: Vec2,

    /// Packed `IM_COL32` values, indexed by [`Col::index`].
    pub colors: [u32; Col::COUNT],
}

impl Style {
    /// `GetColorU32(col)`: the packed colour of a style entry, with `Style::Alpha` folded into its
    /// alpha channel the way `GetColorU32` does when no explicit multiplier is given.
    pub fn color_u32(&self, col: Col) -> u32 {
        let packed = self.colors[col.index()];
        if self.alpha == 1.0 {
            return packed;
        }
        let alpha = (packed >> 24) & 0xFF;
        let scaled = ((alpha as f32 / 255.0) * self.alpha * 255.0 + 0.5).clamp(0.0, 255.0) as u32;
        (packed & 0x00FF_FFFF) | (scaled << 24)
    }

    /// `GetColorU32(col, alpha_mul)`: the packed colour with its alpha scaled, computed the way the
    /// reference does it — through floats and `ColorConvertFloat4ToU32`, which rounds with `+ 0.5`, so
    /// `Text` at `0.70` gives `0xB3`, not `0xB2`. `Style::Alpha` multiplies on top of `alpha_mul`.
    pub fn color_u32_alpha(&self, col: Col, alpha_mul: f32) -> u32 {
        let packed = self.colors[col.index()];
        let alpha = (packed >> 24) & 0xFF;
        let scaled = ((alpha as f32 / 255.0) * alpha_mul * self.alpha * 255.0 + 0.5)
            .clamp(0.0, 255.0) as u32;
        (packed & 0x00FF_FFFF) | (scaled << 24)
    }
}

impl Default for Style {
    /// The reference's default style, as printed by `tools/conformance` (`style_defaults`).
    fn default() -> Self {
        Self {
            window_padding: Vec2 { x: 8.0, y: 8.0 },
            item_spacing: Vec2 { x: 8.0, y: 4.0 },
            frame_padding: Vec2 { x: 4.0, y: 3.0 },
            item_inner_spacing: Vec2 { x: 4.0, y: 4.0 },
            cell_padding: Vec2 { x: 4.0, y: 2.0 },
            grab_min_size: 12.0,
            indent_spacing: 21.0,
            selectable_rounding: 0.0,
            popup_bg: 0xF0242424,
            menu_bar_bg: 0xFF242424,
            table_header_bg: 0xFF4A_4747,
            header_hovered: 0xCCFA_9642,
            header_active: 0xFFFA_9642,
            table_row_bg: 0x0000_0000,
            table_row_bg_alt: 0x0FFF_FFFF,
            table_border_strong: 0xFF73_6666,
            table_border_light: 0xFF57_5252,
            window_min_size: Vec2 { x: 32.0, y: 32.0 },
            scrollbar_size: 14.0,
            separator_size: 1.0,
            separator_text_padding: Vec2 { x: 20.0, y: 3.0 },
            separator_text_border_size: 3.0,
            separator_text_align: Vec2 { x: 0.0, y: 0.5 },
            alpha: 1.0,
            disabled_alpha: 0.6,
            scrollbar_padding: 2.0,
            frame_rounding: 0.0,
            window_rounding: 0.0,
            grab_rounding: 0.0,
            child_rounding: 0.0,
            popup_rounding: 0.0,
            scrollbar_rounding: 9.0,
            tab_rounding: 5.0,
            window_border_size: 1.0,
            window_border_hover_padding: 4.0,
            frame_border_size: 0.0,
            child_border_size: 1.0,
            popup_border_size: 1.0,
            symbol_arrow_size_factor: Vec2 { x: 0.72, y: 0.40 },
            symbol_arrow_thickness_factor: 0.30,
            window_title_align: Vec2 { x: 0.0, y: 0.5 },
            display_window_padding: Vec2 { x: 19.0, y: 19.0 },
            display_safe_area_padding: Vec2 { x: 3.0, y: 3.0 },
            colors: [
                0xFFFF_FFFF, // Text
                0xFF94_9494, // TextDisabled
                0xFF26_2626, // WindowBg
                0xFF1A_1A1A, // FrameBg
                0xFF4F_4030, // FrameBgHovered
                0xFF78_5436, // FrameBgActive
                0x66FA_9642, // Button
                0xFFFA_9642, // ButtonHovered
                0xFFFA_870F, // ButtonActive
                0xFFFA_9642, // CheckMark
                0xFFE0_853D, // SliderGrab
                0xFFFA_9642, // SliderGrabActive
                0x8080_6E6E, // Border
                0x8080_6E6E, // Separator
                0x4FFA_9642, // Header
                0xCCFA_9642, // HeaderHovered
                0xFFFA_9642, // HeaderActive
                0xFF0A_0A0A, // TitleBg (pinned from the reference)
                0xFF7A_4A29, // TitleBgActive (pinned from the reference)
                0x8200_0000, // TitleBgCollapsed (IM_COL32(0, 0, 0, 130), pinned)
                0x8705_0505, // ScrollbarBg (pinned from the reference)
                0xFF4F_4F4F, // ScrollbarGrab (pinned from the reference)
                0xFF69_6969, // ScrollbarGrabHovered (pinned from the reference)
                0xFF82_8282, // ScrollbarGrabActive (pinned from the reference)
                0x33FA_9642, // ResizeGrip (pinned from the reference)
                0xABFA_9642, // ResizeGripHovered (pinned from the reference)
                0xF2FA_9642, // ResizeGripActive (pinned from the reference)
                0xC7BF_661A, // SeparatorHovered (pinned from the reference)
                0xFFBF_661A, // SeparatorActive (pinned from the reference)
                0x59FA_9642, // TextSelectedBg (pinned from the reference)
                0xFFFF_FFFF, // InputTextCursor (pinned from the reference)
            ],
        }
    }
}
