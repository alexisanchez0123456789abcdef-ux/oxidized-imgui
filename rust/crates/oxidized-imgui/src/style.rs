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
    TitleBgActive,
    HeaderHovered,
    HeaderActive,
}

impl Col {
    pub const COUNT: usize = 18;

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
            Col::TitleBgActive => 17,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Style {
    pub window_padding: Vec2,
    pub item_spacing: Vec2,
    pub frame_padding: Vec2,
    pub item_inner_spacing: Vec2,
    /// `Style::GrabMinSize`: the grab of a slider.
    pub grab_min_size: f32,

    pub frame_rounding: f32,
    pub window_rounding: f32,
    pub grab_rounding: f32,
    pub child_rounding: f32,
    pub popup_rounding: f32,
    pub scrollbar_rounding: f32,
    pub tab_rounding: f32,

    pub window_border_size: f32,
    pub frame_border_size: f32,
    pub child_border_size: f32,
    pub popup_border_size: f32,

    /// Packed `IM_COL32` values, indexed by [`Col::index`].
    pub colors: [u32; Col::COUNT],
}

impl Style {
    /// `GetColorU32(col)`: the packed colour of a style entry.
    pub fn color_u32(&self, col: Col) -> u32 {
        self.colors[col.index()]
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
            grab_min_size: 12.0,
            frame_rounding: 0.0,
            window_rounding: 0.0,
            grab_rounding: 0.0,
            child_rounding: 0.0,
            popup_rounding: 0.0,
            scrollbar_rounding: 9.0,
            tab_rounding: 5.0,
            window_border_size: 1.0,
            frame_border_size: 0.0,
            child_border_size: 1.0,
            popup_border_size: 1.0,
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
                0xFF7A_4A29, // TitleBgActive (pinned from the reference)
            ],
        }
    }
}
