//! Style defaults.
//!
//! Values from the reference's `ImGuiStyle` constructor (1.93.1): they decide the geometry of
//! every window and item, so they are part of the compatibility contract.

use crate::draw_list::Vec2;

/// `ImGuiStyle`: only the fields whose values affect the geometry implemented so far.
#[derive(Clone, Debug)]
pub struct Style {
    pub window_padding: Vec2,
    pub item_spacing: Vec2,
    pub frame_padding: Vec2,
    pub item_inner_spacing: Vec2,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            window_padding: Vec2 { x: 8.0, y: 8.0 },
            item_spacing: Vec2 { x: 8.0, y: 4.0 },
            frame_padding: Vec2 { x: 4.0, y: 3.0 },
            item_inner_spacing: Vec2 { x: 4.0, y: 4.0 },
        }
    }
}
