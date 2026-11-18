//! Input/output state and the mouse event model.
//!
//! What the reference's `UpdateMouseInputs` computes from the raw state the platform feeds in:
//! rounded mouse position, per-frame delta, clicks and releases derived from the down duration,
//! and the double-click counter. Keyboard, wheel and drag-lock are not implemented yet.

use crate::draw_list::Vec2;

/// `IM_ARRAYSIZE(io.MouseDown)`: left, right, middle, extra 1, extra 2.
pub const BUTTONS: usize = 5;

/// The keyboard keys this crate models. The reference has a much larger `ImGuiKey` enum; this is
/// the subset the conformance scenarios drive, and it grows as widgets need more.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Key {
    A,
    LeftCtrl,
    RightCtrl,
    LeftShift,
    LeftAlt,
    LeftSuper,
    /// `ImGuiMod_Ctrl`: the modifier the backend reports, which is what `io.KeyCtrl` derives from.
    ModCtrl,
    ModShift,
    ModAlt,
    ModSuper,
}

impl Key {
    pub const COUNT: usize = 10;

    pub(crate) fn index(self) -> usize {
        match self {
            Key::A => 0,
            Key::LeftCtrl => 1,
            Key::RightCtrl => 2,
            Key::LeftShift => 3,
            Key::LeftAlt => 4,
            Key::LeftSuper => 5,
            Key::ModCtrl => 6,
            Key::ModShift => 7,
            Key::ModAlt => 8,
            Key::ModSuper => 9,
        }
    }
}

/// `ImGuiKeyData`: what the reference keeps per key.
#[derive(Copy, Clone, Debug)]
pub struct KeyData {
    pub down: bool,
    pub analog_value: f32,
    pub down_duration: f32,
}

impl Default for KeyData {
    fn default() -> Self {
        Self {
            down: false,
            analog_value: 0.0,
            down_duration: -1.0,
        }
    }
}

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
    /// `ImGuiIO::MouseDragThreshold`: how far the mouse must travel before it counts as dragging.
    /// io.MouseWheel / io.MouseWheelH: filled from the wheel events of the frame.
    pub mouse_wheel: f32,
    pub mouse_wheel_h: f32,
    /// io.ConfigScrollbarScrollByPage: a click on a scrollbar's track scrolls a page instead of
    /// jumping to the clicked position.
    pub config_scrollbar_scroll_by_page: bool,
    pub mouse_drag_threshold: f32,
    /// `ImGuiIO::MouseDragMaxDistanceSqr`: the furthest the mouse went since it was pressed,
    /// squared. It is what turns a held button into a drag.
    pub mouse_drag_max_distance_sqr: [f32; BUTTONS],

    /// Per-key state, indexed by `Key::index`.
    pub keys: [KeyData; Key::COUNT],
    /// Events queued by `add_key_event`, applied on the next `begin_frame`.
    pub(crate) key_queue: Vec<(Key, bool)>,
    /// `io.KeyCtrl`, derived from the physical modifier keys.
    pub key_ctrl: bool,
    pub key_shift: bool,
    pub key_alt: bool,
    pub key_super: bool,

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
            config_scrollbar_scroll_by_page: false,
            mouse_wheel: 0.0,
            mouse_wheel_h: 0.0,
            mouse_drag_threshold: 6.0,
            mouse_drag_max_distance_sqr: [0.0; BUTTONS],
            keys: [KeyData::default(); Key::COUNT],
            key_queue: Vec::new(),
            key_ctrl: false,
            key_shift: false,
            key_alt: false,
            key_super: false,
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

    /// `ImGuiIO::AddKeyEvent`: queues a key event, dropping it when the state does not change.
    pub fn add_key_event(&mut self, key: Key, down: bool) {
        let index = key.index();
        let queued = self
            .key_queue
            .iter()
            .rev()
            .find(|(queued_key, _)| *queued_key == key)
            .map(|(_, queued_down)| *queued_down);
        let current_down = queued.unwrap_or(self.keys[index].down);
        if current_down == down {
            return;
        }
        self.key_queue.push((key, down));
    }

    pub fn key_data(&self, key: Key) -> &KeyData {
        &self.keys[key.index()]
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        self.keys[key.index()].down
    }

    /// `IsKeyPressed(key, repeat = false)`: true only on the frame the key went down.
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys[key.index()].down_duration == 0.0
    }

    /// `IsMouseDragging(button, lock_threshold)`: the button is down **and** the mouse has moved
    /// past the threshold since it was pressed. A negative threshold means the IO default.
    pub fn is_mouse_dragging(&self, button: usize, lock_threshold: f32) -> bool {
        if !self.mouse_down[button] {
            return false;
        }
        let threshold = if lock_threshold < 0.0 {
            self.mouse_drag_threshold
        } else {
            lock_threshold
        };
        self.mouse_drag_max_distance_sqr[button] >= threshold * threshold
    }

    /// `GetMouseDragDelta(button, lock_threshold)`: stays zero until the drag threshold is passed,
    /// then it is the distance from the position where the button went down.
    pub fn mouse_drag_delta(&self, button: usize, lock_threshold: f32) -> Vec2 {
        let threshold = if lock_threshold < 0.0 {
            self.mouse_drag_threshold
        } else {
            lock_threshold
        };
        if !(self.mouse_down[button] || self.mouse_released[button]) {
            return Vec2::default();
        }
        if self.mouse_drag_max_distance_sqr[button] < threshold * threshold {
            return Vec2::default();
        }
        let Some(pos) = self.mouse_pos else {
            return Vec2::default();
        };
        Vec2 {
            x: pos.x - self.mouse_clicked_pos[button].x,
            y: pos.y - self.mouse_clicked_pos[button].y,
        }
    }
}
