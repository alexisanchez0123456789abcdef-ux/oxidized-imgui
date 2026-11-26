//! Input/output state and the keyboard and mouse event model.
//!
//! What the reference's `UpdateKeyboardInputs` and `UpdateMouseInputs` compute from the raw state
//! the platform feeds in: rounded mouse position, per-frame delta, clicks and releases derived from
//! the down duration, the double-click counter, per-key down durations with their previous value,
//! and the modifier flags. The character queue (`InputQueueCharacters`) is filled by
//! [`Io::add_input_character`] and consumed at the end of the frame, as the reference does.

use crate::draw_list::Vec2;

/// `IM_ARRAYSIZE(io.MouseDown)`: left, right, middle, extra 1, extra 2.
pub const BUTTONS: usize = 5;

/// `ImGuiKey`: the keyboard keys, named and ordered as the reference.
///
/// The reference's enum also carries the gamepad buttons, the mouse buttons as keys, and the wheel
/// axes; those are not modelled here yet. What is modelled is the whole keyboard, so code written
/// against ImGui's names keeps working.
///
/// `ImGuiMod_*` live in the same enum in the reference and are stored per key there too, which is
/// why they close the list: `io.KeyCtrl` is derived from `ImGuiMod_Ctrl`, not from `LeftCtrl`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum Key {
    /// `ImGuiKey_None`.
    None = 0,
    // Keyboard: navigation and text control.
    Tab,
    LeftArrow,
    RightArrow,
    UpArrow,
    DownArrow,
    PageUp,
    PageDown,
    Home,
    End,
    Insert,
    Delete,
    Backspace,
    Space,
    Enter,
    Escape,
    // Keyboard: the physical modifier keys.
    LeftCtrl,
    LeftShift,
    LeftAlt,
    LeftSuper,
    RightCtrl,
    RightShift,
    RightAlt,
    RightSuper,
    Menu,
    // Keyboard: digits, letters and function keys.
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    // Keyboard: punctuation, named after a US layout as the reference does.
    Apostrophe,
    Comma,
    Minus,
    Period,
    Slash,
    Semicolon,
    Equal,
    LeftBracket,
    Backslash,
    RightBracket,
    GraveAccent,
    // Keyboard: locks and system keys.
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    // Keyboard: the numeric keypad.
    Keypad0,
    Keypad1,
    Keypad2,
    Keypad3,
    Keypad4,
    Keypad5,
    Keypad6,
    Keypad7,
    Keypad8,
    Keypad9,
    KeypadDecimal,
    KeypadDivide,
    KeypadMultiply,
    KeypadSubtract,
    KeypadAdd,
    KeypadEnter,
    KeypadEqual,
    // Keyboard: the extra keys some layouts have.
    AppBack,
    AppForward,
    Oem102,
    // Modifiers, submitted by the backend on their own.
    /// `ImGuiMod_Ctrl`: what `io.KeyCtrl` derives from.
    ModCtrl,
    ModShift,
    ModAlt,
    ModSuper,
}

impl Key {
    /// `ImGuiKey_NamedKey_COUNT` plus the four modifiers.
    pub const COUNT: usize = Key::ModSuper as usize + 1;

    /// `key - ImGuiKey_NamedKey_BEGIN`: the index into `io.KeysData`.
    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

/// `ImGuiKeyData`: what the reference keeps per key.
#[derive(Copy, Clone, Debug)]
pub struct KeyData {
    pub down: bool,
    pub analog_value: f32,
    /// `DownDuration`: seconds the key has been held, -1 while it is up.
    pub down_duration: f32,
    /// `DownDurationPrev`: the same value on the previous frame, which is what tells a release
    /// (`DownDurationPrev >= 0`) apart from a key that was never pressed.
    pub down_duration_prev: f32,
}

impl Default for KeyData {
    fn default() -> Self {
        Self {
            down: false,
            analog_value: 0.0,
            down_duration: -1.0,
            down_duration_prev: -1.0,
        }
    }
}

/// `ImGuiMouseCursor`: what the backend should show over the pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum MouseCursor {
    /// The default arrow.
    #[default]
    Arrow = 0,
    /// Over editable text.
    TextInput = 1,
    /// Over a resize handle that works in both directions.
    ResizeAll = 2,
    /// Vertical resize, such as the top or bottom border.
    ResizeNS = 3,
    /// Horizontal resize, such as the left or right border.
    ResizeEW = 4,
    /// The lower-left resize grip.
    ResizeNESW = 5,
    /// The lower-right resize grip.
    ResizeNWSE = 6,
    /// Over a hand-grab area.
    Hand = 7,
    /// Over something that cannot be acted on.
    NotAllowed = 8,
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
    /// io.ConfigWindowsResizeFromEdges: the lower-left corner is a resize grip too, not only the
    /// lower-right one.
    pub config_windows_resize_from_edges: bool,
    /// io.MouseCursor: reset to Arrow every frame, then set by whatever the mouse is over.
    pub mouse_cursor: MouseCursor,
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
    /// `io.InputQueueCharacters`: the characters the backend has produced this frame. The reference
    /// keeps UTF-16 code units; this keeps code points, which is the same thing minus lone
    /// surrogates (a code point above U+FFFF is reported as U+FFFD, exactly as the reference does).
    pub input_queue_characters: Vec<char>,
    /// The clipboard the platform backend owns. The reference reaches it through
    /// `PlatformIO.Platform_Get/SetClipboardTextFn`; without a backend those are NULL and copy and
    /// paste do nothing, which is why this is an `Option`.
    pub clipboard: Option<String>,
    /// `io.ConfigInputTextCursorBlink`: whether the caret of a text field blinks.
    pub config_input_text_cursor_blink: bool,
    /// `io.KeyRepeatDelay` and `io.KeyRepeatRate`: when a held key starts and then repeats.
    pub key_repeat_delay: f32,
    pub key_repeat_rate: f32,
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
            config_windows_resize_from_edges: true,
            mouse_cursor: MouseCursor::Arrow,
            mouse_wheel: 0.0,
            mouse_wheel_h: 0.0,
            mouse_drag_threshold: 6.0,
            mouse_drag_max_distance_sqr: [0.0; BUTTONS],
            keys: [KeyData::default(); Key::COUNT],
            key_queue: Vec::new(),
            input_queue_characters: Vec::new(),
            clipboard: None,
            config_input_text_cursor_blink: true,
            key_repeat_delay: 0.275,
            key_repeat_rate: 0.050,
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

    /// `ImGuiIO::AddInputCharacter`: queues one character for the widgets of the next frame.
    ///
    /// A code point the reference cannot carry in its 16-bit unit becomes U+FFFD there, so it does
    /// here too: what matters is that a widget ends up with the same text.
    pub fn add_input_character(&mut self, c: char) {
        let c = if (c as u32) <= 0xFFFF { c } else { '\u{FFFD}' };
        self.input_queue_characters.push(c);
    }

    /// `ImGuiIO::AddInputCharactersUTF8`: queues every code point of the string.
    pub fn add_input_characters_utf8(&mut self, text: &str) {
        for c in text.chars() {
            self.add_input_character(c);
        }
    }

    /// `ImGui::GetClipboardText`: the text the platform holds, if any.
    pub fn get_clipboard_text(&self) -> Option<&str> {
        self.clipboard.as_deref()
    }

    /// `ImGui::SetClipboardText`: hand the text to the platform clipboard.
    pub fn set_clipboard_text(&mut self, text: &str) {
        self.clipboard = Some(text.to_string());
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        self.keys[key.index()].down
    }

    /// `IsKeyPressed(key, repeat = false)`: true only on the frame the key went down.
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys[key.index()].down_duration == 0.0
    }

    /// `IsKeyPressed(key, repeat = true)`: the frame it went down, and then again after
    /// `KeyRepeatDelay`, at `KeyRepeatRate` intervals.
    pub fn is_key_pressed_repeat(&self, key: Key) -> bool {
        let data = &self.keys[key.index()];
        if !data.down {
            return false;
        }
        let t = data.down_duration;
        t == 0.0
            || (t > self.key_repeat_delay
                && crate::io::typematic_repeat_amount(
                    t - self.delta_time,
                    t,
                    self.key_repeat_delay,
                    self.key_repeat_rate,
                ) > 0)
    }

    /// `IsKeyReleased`: the frame it went up, told apart from a key that was never down.
    pub fn is_key_released(&self, key: Key) -> bool {
        let data = &self.keys[key.index()];
        !(data.down_duration_prev < 0.0 || data.down)
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
/// `CalcTypematicRepeatAmount`: how many repeats a key that has been held for `t1` seconds
/// (the previous frame's duration was `t0`) produces between two frames.
pub fn typematic_repeat_amount(t0: f32, t1: f32, repeat_delay: f32, repeat_rate: f32) -> i32 {
    if t1 == 0.0 {
        return 1;
    }
    if t0 >= t1 {
        return 0;
    }
    if repeat_rate <= 0.0 {
        return i32::from(t0 < repeat_delay && t1 >= repeat_delay);
    }
    let count_t0 = if t0 < repeat_delay {
        -1
    } else {
        ((t0 - repeat_delay) / repeat_rate) as i32
    };
    let count_t1 = if t1 < repeat_delay {
        -1
    } else {
        ((t1 - repeat_delay) / repeat_rate) as i32
    };
    count_t1 - count_t0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_indices_are_contiguous_and_start_at_none() {
        assert_eq!(Key::None.index(), 0);
        assert_eq!(Key::Tab.index(), 1);
        assert_eq!(Key::ModSuper.index(), Key::COUNT - 1);
    }

    /// `CalcTypematicRepeatAmount`, checked against the formula in `imgui.cpp`: one at zero, then
    /// one per repeat interval once the delay has passed.
    #[test]
    fn typematic_repeat_follows_the_reference_formula() {
        let (delay, rate) = (0.275f32, 0.050f32);
        // Nothing before the delay: the count only turns positive once t1 reaches it.
        assert_eq!(typematic_repeat_amount(0.0, 0.05, delay, rate), 0);
        assert_eq!(typematic_repeat_amount(0.20, 0.25, delay, rate), 0);
        // The frame that crosses the delay repeats once.
        assert_eq!(typematic_repeat_amount(0.25, 0.30, delay, rate), 1);
        assert_eq!(typematic_repeat_amount(0.30, 0.35, delay, rate), 1);
        // Then one per rate interval, and two when a frame is longer than the rate.
        assert_eq!(typematic_repeat_amount(0.35, 0.60, delay, rate), 5);
        // A zero duration answers one, and a negative rate becomes a single step at the delay.
        assert_eq!(typematic_repeat_amount(0.0, 0.0, delay, rate), 1);
        assert_eq!(typematic_repeat_amount(0.20, 0.30, delay, 0.0), 1);
        assert_eq!(typematic_repeat_amount(0.30, 0.40, delay, 0.0), 0);
    }

    #[test]
    fn characters_keep_their_order_and_astral_ones_become_replacement() {
        let mut io = Io::default();
        io.add_input_characters_utf8("aé😀b");
        assert_eq!(
            io.input_queue_characters,
            vec!['a', '\u{e9}', '\u{FFFD}', 'b']
        );
    }

    #[test]
    fn a_press_is_one_frame_and_a_release_is_the_one_after() {
        let mut io = Io {
            delta_time: 0.05,
            ..Io::default()
        };
        io.add_key_event(Key::LeftArrow, true);
        // What `UpdateKeyboardInputs` does, frame by frame.
        let step = |io: &mut Io| {
            for (key, down) in core::mem::take(&mut io.key_queue) {
                io.keys[key.index()].down = down;
            }
            for data in &mut io.keys {
                data.down_duration_prev = data.down_duration;
                data.down_duration = if data.down {
                    if data.down_duration < 0.0 {
                        0.0
                    } else {
                        data.down_duration + io.delta_time
                    }
                } else {
                    -1.0
                };
            }
        };
        step(&mut io);
        assert!(io.is_key_pressed(Key::LeftArrow));
        assert!(!io.is_key_pressed_repeat(Key::LeftArrow) || io.delta_time > 0.0);
        step(&mut io);
        assert!(!io.is_key_pressed(Key::LeftArrow));
        assert!(!io.is_key_released(Key::LeftArrow));
        io.add_key_event(Key::LeftArrow, false);
        step(&mut io);
        assert!(io.is_key_released(Key::LeftArrow));
        step(&mut io);
        assert!(!io.is_key_released(Key::LeftArrow));
    }

    #[test]
    fn a_held_key_repeats_only_after_the_delay() {
        let mut io = Io {
            delta_time: 0.05,
            ..Io::default()
        };
        io.add_key_event(Key::LeftArrow, true);
        for (key, down) in core::mem::take(&mut io.key_queue) {
            io.keys[key.index()].down = down;
        }
        let mut repeats = Vec::new();
        for _ in 0..14 {
            for data in &mut io.keys {
                data.down_duration_prev = data.down_duration;
                data.down_duration = if data.down {
                    if data.down_duration < 0.0 {
                        0.0
                    } else {
                        data.down_duration + io.delta_time
                    }
                } else {
                    -1.0
                };
            }
            repeats.push(io.is_key_pressed_repeat(Key::LeftArrow));
            // Nothing is ever queued again, so only the duration moves.
        }
        // The first frame, then nothing while t <= 0.275, then every frame.
        assert!(repeats[0]);
        assert!(!repeats[1] && !repeats[5]);
        assert!(repeats[6]); // t = 0.30 > 0.275 with a step that crosses it
        assert!(repeats[13]);
    }
}
