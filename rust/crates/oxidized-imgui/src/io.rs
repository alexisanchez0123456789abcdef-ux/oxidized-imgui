//! Input/output state and the mouse event model.
//!
//! What the reference's `UpdateMouseInputs` computes from the raw state the platform feeds in:
//! rounded mouse position, per-frame delta, clicks and releases derived from the down duration,
//! and the double-click counter. Keyboard, wheel and drag-lock are not implemented yet.

use crate::draw_list::Vec2;

/// `IM_ARRAYSIZE(io.MouseDown)`: left, right, middle, extra 1, extra 2.
pub const BUTTONS: usize = 5;

/// `ImGuiIO`, restricted to what this crate models.
#[derive(Clone, Debug)]
pub struct Io {
    pub display_size: Vec2,
    /// Seconds since the previous frame (`ImGuiIO::DeltaTime`).
    pub delta_time: f32,
    /// `None` means "no mouse position" (the reference uses `-FLT_MAX`).
    pub mouse_pos: Option<Vec2>,
    pub mouse_down: [bool; BUTTONS],

    // Outputs, updated by `Context::begin_frame`.
    pub mouse_pos_prev: Option<Vec2>,
    pub mouse_delta: Vec2,
    pub mouse_clicked: [bool; BUTTONS],
    pub mouse_released: [bool; BUTTONS],
    pub mouse_down_duration: [f32; BUTTONS],
    /// 0 except on frames where the button was clicked, as the reference.
    pub mouse_clicked_count: [i32; BUTTONS],

    pub mouse_double_click_time: f32,
    pub mouse_double_click_max_dist: f32,

    pub(crate) mouse_clicked_time: [f64; BUTTONS],
    pub(crate) mouse_clicked_pos: [Vec2; BUTTONS],
    pub(crate) mouse_clicked_last_count: [i32; BUTTONS],
}

impl Default for Io {
    fn default() -> Self {
        Self {
            display_size: Vec2 {
                x: 1280.0,
                y: 720.0,
            },
            delta_time: 1.0 / 60.0,
            mouse_pos: None,
            mouse_down: [false; BUTTONS],
            mouse_pos_prev: None,
            mouse_delta: Vec2::default(),
            mouse_clicked: [false; BUTTONS],
            mouse_released: [false; BUTTONS],
            mouse_down_duration: [-1.0; BUTTONS],
            mouse_clicked_count: [0; BUTTONS],
            mouse_double_click_time: 0.30,
            mouse_double_click_max_dist: 6.0,
            mouse_clicked_time: [0.0; BUTTONS],
            mouse_clicked_pos: [Vec2::default(); BUTTONS],
            mouse_clicked_last_count: [0; BUTTONS],
        }
    }
}

impl Io {
    /// `IsMousePosValid`.
    pub fn is_mouse_pos_valid(&self) -> bool {
        self.mouse_pos.is_some()
    }
}
