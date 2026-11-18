//! Context, windows and cursor layout.
//!
//! The part of the reference that decides where things go: a window with an explicit position and
//! size, its content region, and a cursor advancing over items (`Dummy`, `SameLine`). Text
//! metrics, widgets, scrolling, stacking and settings are not implemented yet.
//!
//! Numbers here are verified against the reference by the `window_layout` conformance scenario.

use crate::draw_list::Vec2;
use crate::font::Font;
use crate::id::{ImGuiID, hash_str_utf8};
use crate::io::{BUTTONS, Io, Key};
use crate::style::Style;

/// `g_ImGuiDirCosSin[ImGuiDir_Down]`: the arrow a window shows while it is open.
const ARROW_DOWN_COS: f32 = 0.0;
const ARROW_DOWN_SIN: f32 = 1.0;
/// `g_ImGuiDirCosSin[ImGuiDir_Right]`: the arrow a collapsed window shows.
const ARROW_RIGHT_COS: f32 = 1.0;
const ARROW_RIGHT_SIN: f32 = 0.0;

/// The per-window flags the port models, mirroring the `ImGuiWindowFlags` bits of the same name.
///
/// They used to be one `set_next_window_*` call per flag, which does not scale: this is the
/// reference's own shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowFlags(u32);

impl WindowFlags {
    pub const NONE: Self = Self(0);
    /// `ImGuiWindowFlags_NoMove`: a click cannot start a move drag on this window.
    pub const NO_MOVE: Self = Self(1 << 0);
    /// `ImGuiWindowFlags_NoScrollbar`: no scrollbar, even when the content does not fit.
    pub const NO_SCROLLBAR: Self = Self(1 << 1);
    /// `ImGuiWindowFlags_HorizontalScrollbar`: a horizontal scrollbar may show up too.
    pub const HORIZONTAL_SCROLLBAR: Self = Self(1 << 2);
    /// `ImGuiWindowFlags_AlwaysAutoResize`: the window sizes itself to its content.
    pub const ALWAYS_AUTO_RESIZE: Self = Self(1 << 3);
    /// `ImGuiWindowFlags_NoResize`: no resize grips and no resize borders.
    pub const NO_RESIZE: Self = Self(1 << 4);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl core::ops::BitOr for WindowFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for WindowFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Everything the port remembers about a window between frames, so `Begin` can start from it.
#[derive(Clone, Debug)]
struct WindowMemory {
    name: String,
    pos: Vec2,
    size_full: Vec2,
    collapsed: bool,
    want_collapse_toggle: bool,
    scroll: Vec2,
    /// `window->ContentSize` of the previous frame: the scroll maximum and the auto-fit size use it.
    content_size: Vec2,
    /// The interior size of the previous frame, before the scrollbars took their share: the
    /// scrollbar decision compares the content against this, not against the current size.
    inner_size: Vec2,
    flags: WindowFlags,
}

impl WindowMemory {
    fn find<'a>(memory: &'a [WindowMemory], name: &str) -> Option<&'a WindowMemory> {
        memory.iter().find(|known| known.name == name)
    }

    fn find_mut<'a>(memory: &'a mut [WindowMemory], name: &str) -> Option<&'a mut WindowMemory> {
        memory.iter_mut().find(|known| known.name == name)
    }
}

/// What `begin_window` has to know about the window it is opening.
struct WindowOptions<'a> {
    /// `None` for a window without a title bar.
    title: Option<&'a str>,
    /// `ImGuiWindowFlags_NoBackground` when false.
    background: bool,
    /// `p_open != NULL`: the title bar gets a close button, and a press turns it off.
    open: Option<&'a mut bool>,
    /// `ImGuiWindowFlags_NoCollapse` when false: the title bar gets a collapse arrow.
    collapsible: bool,
    /// The flags of the window, `NoMove` and `NoScrollbar` included.
    flags: WindowFlags,
}

/// An open window. Only the geometry the harness checks is modelled.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub name: String,
    pub pos: Vec2,
    /// The size the window displays: the title bar alone while it is collapsed.
    pub size: Vec2,
    /// `SizeFull`: what the window is when it is not collapsed.
    pub size_full: Vec2,
    pub collapsed: bool,
    pub want_collapse_toggle: bool,
    /// The window's ImGuiWindowFlags, as it was opened this frame.
    pub flags: WindowFlags,
    /// `window->Scroll`: how far the content is scrolled inside the window.
    pub scroll: Vec2,
    /// `window->ScrollMax`: the largest scroll the content allows.
    pub scroll_max: Vec2,
    /// `window->ContentSize`: how big the content was on the previous frame.
    pub content_extent: Vec2,
    /// `window->ScrollbarY`: whether the vertical scrollbar is shown this frame.
    pub scrollbar_y: bool,
    /// `window->ScrollbarX`: whether the horizontal scrollbar is shown this frame.
    pub scrollbar_x: bool,
    /// `window->InnerRect`: the window rectangle without its decorations.
    pub inner_rect: (Vec2, Vec2),
    /// `pos + style.WindowPadding`, minus the scroll.
    pub content_origin: Vec2,
    /// `size - style.WindowPadding * 2`, what `GetContentRegionAvail` returns inside the window.
    pub content_size: Vec2,
    cursor: Vec2,
    cursor_start: Vec2,
    cursor_max: Vec2,
    line_start_x: f32,
    line_height: f32,
    last_min: Vec2,
    last_max: Vec2,
    has_item: bool,
}

/// Frame state, identifier stack, style, input state and the open window stack.
#[derive(Debug)]
pub struct Context {
    id_stack: Vec<ImGuiID>,
    frame_count: u64,
    last_frame_draw_calls: usize,
    style: Style,
    font: Font,
    /// `ImDrawListSharedData::TexUvLines`, shared with the frame's draw list.
    line_uv_lines: Vec<[f32; 4]>,
    io: Io,
    /// `g.Time`: the reference keeps it as a `double` and `GetTime()` returns it as such.
    time: f64,
    windows: Vec<Window>,
    /// Per-window state between frames: position, size, scroll, content size and flags.
    memory: Vec<WindowMemory>,
    /// `SetNextWindowScroll` for the window about to start.
    next_scroll: Option<Vec2>,
    /// g.ScrollbarClickDeltaToGrabCenter: where inside the grab the seek started.
    scrollbar_click_delta: f32,
    /// g.ScrollbarSeekMode: 0 seeks to the click, +-1 scrolls a page in that direction.
    scrollbar_seek_mode: i32,
    /// g.ActiveIdIsJustActivated, as far as the scrollbar needs it.
    active_id_just_activated: bool,
    /// g.WheelingWindow: the window the wheel is locked to, so it keeps scrolling while the
    /// mouse wanders over another one.
    wheeling_window: Option<String>,
    /// g.WheelingWindowRefMousePos: where the mouse was when the lock was taken.
    wheeling_ref_mouse: Vec2,
    /// g.WheelingWindowReleaseTimer: seconds of lock left.
    wheeling_release_timer: f32,
    /// The flags of the window about to start, set with `set_next_window_flags`.
    next_flags: Option<WindowFlags>,
    /// `g.Windows`, back to front: the order the lists are rendered in. It survives frames.
    display_order: Vec<String>,
    /// Names of the windows that were submitted on the previous frame (`WasActive`).
    last_frame_windows: Vec<String>,
    /// The lists closed this frame, by name, in the order their windows were opened.
    frame_lists: Vec<(String, crate::draw_list::DrawList)>,
    /// The list of the last window that closed, so `draw_list()` still answers outside a window.
    last_list: crate::draw_list::DrawList,
    /// `g.HoveredWindow`: the top-most window under the mouse, recomputed each frame.
    hovered_window: Option<String>,
    /// `g.NavWindow` as far as the port needs it: the window whose title bar is highlighted.
    focused_window: Option<String>,
    /// `g.HoveredId`: the widget under the mouse, which blocks the click-to-raise.
    hovered_id: Option<ImGuiID>,
    active_id: Option<ImGuiID>,
    /// `ActiveIdClickOffset`: where inside the window (or the grip) the drag started.
    active_click_offset: Vec2,
    /// `resize_grip_col[2]`: the colour each lower corner grip paints this frame, zero when hidden.
    resize_grip_cols: [u32; 2],
    /// `g.HoveredIdTimer`: how long the border under the mouse has been hovered. The reference
    /// waits WINDOWS_RESIZE_FROM_EDGES_FEEDBACK_TIMER before it reports a border, so brushing past
    /// an edge on the way to a widget does not light it up.
    resize_hover_timer: f32,
    /// The border hovered last frame, so the timer knows when the hover started.
    resize_border_prev: i32,
    /// `g.MovingWindow`: the window that follows the mouse while the move drag lasts.
    moving_window: Option<String>,
    /// The window's draw list: what this frame has drawn so far.
    draw_list: crate::draw_list::DrawList,
    slider_just_activated: bool,
    slider_grab_offset: f32,
    last_item_hovered: bool,
    last_item_active: bool,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            id_stack: Vec::new(),
            frame_count: 0,
            last_frame_draw_calls: 0,
            style: Style::default(),
            font: Font::reference_default(),
            line_uv_lines: Vec::new(),
            io: Io::default(),
            time: 0.0,
            windows: Vec::new(),
            memory: Vec::new(),
            next_scroll: None,
            next_flags: None,
            wheeling_window: None,
            wheeling_ref_mouse: Vec2::default(),
            wheeling_release_timer: 0.0,
            scrollbar_click_delta: 0.0,
            scrollbar_seek_mode: 0,
            active_id_just_activated: false,
            display_order: Vec::new(),
            last_frame_windows: Vec::new(),
            frame_lists: Vec::new(),
            last_list: crate::draw_list::DrawList::new(),
            hovered_window: None,
            focused_window: None,

            hovered_id: None,
            active_id: None,
            active_click_offset: Vec2::default(),
            resize_grip_cols: [0; 2],
            resize_hover_timer: 0.0,
            resize_border_prev: -1,
            moving_window: None,
            draw_list: crate::draw_list::DrawList::new(),
            slider_just_activated: false,
            slider_grab_offset: 0.0,
            last_item_hovered: false,
            last_item_active: false,
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Self {
            style: Style::default(),
            ..Self::default()
        }
    }

    pub fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }

    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Starts a frame: advances the clock, turns the raw input into events, and (as the reference
    /// also does) leaves the identifier stack untouched.
    pub fn begin_frame(&mut self) {
        self.frame_count += 1;
        // A window's draw list is not a bare list: the reference gives it `InitialDrawFlags`,
        // which turn on anti-aliased fills, lines and (textured) corners.
        self.draw_list = self.new_window_draw_list();
        self.frame_lists.clear();
        // g.MouseCursor = ImGuiMouseCursor_Arrow: whatever the last frame asked for is gone.
        self.io.mouse_cursor = crate::io::MouseCursor::Arrow;
        self.resize_grip_cols = [0; 2];
        self.hovered_id = None;
        self.time += self.io.delta_time as f64;
        self.update_keyboard_inputs();
        self.update_mouse_inputs();
        self.update_hovered_window();
        self.update_wheeling_window_lock();
        // `UpdateMouseMovingWindowNewFrame`: the window being dragged follows the mouse, and the
        // drag ends the moment the button comes up.
        self.update_mouse_moving_window_new_frame();
    }

    /// `FindHoveredWindowEx` for top-level windows: walk the display order from the front and take
    /// the first window that was active last frame and whose rectangle contains the mouse.
    ///
    /// A window being dragged is hovered by definition, which is what the reference does too.
    fn update_hovered_window(&mut self) {
        if let Some(moving) = &self.moving_window {
            self.hovered_window = Some(moving.clone());
            return;
        }
        let Some(mouse) = self.io.mouse_pos else {
            self.hovered_window = None;
            return;
        };
        let mut found = None;
        for name in self.display_order.iter().rev() {
            if !self.last_frame_windows.contains(name) {
                continue;
            }
            let Some(memory) = WindowMemory::find(&self.memory, name) else {
                continue;
            };
            let height = if memory.collapsed {
                self.frame_height()
            } else {
                memory.size_full.y
            };
            if mouse.x >= memory.pos.x
                && mouse.x < memory.pos.x + memory.size_full.x
                && mouse.y >= memory.pos.y
                && mouse.y < memory.pos.y + height
            {
                found = Some(name.clone());
                break;
            }
        }
        self.hovered_window = found;
    }

    /// The window the mouse is over, as `ImGui::GetHoveredWindow` would report it.
    pub fn hovered_window_name(&self) -> Option<&str> {
        self.hovered_window.as_deref()
    }

    /// `g.WheelingWindow`: the window the wheel is currently locked to.
    pub fn wheeling_window_name(&self) -> Option<&str> {
        self.wheeling_window.as_deref()
    }

    /// The first half of `UpdateMouseWheel`: the lock ages with the frame and is dropped when the
    /// mouse has moved further than the drag threshold, or when its timer runs out.
    fn update_wheeling_window_lock(&mut self) {
        let Some(name) = self.wheeling_window.clone() else {
            return;
        };
        let _ = name;
        self.wheeling_release_timer -= self.io.delta_time;
        if let Some(mouse) = self.io.mouse_pos {
            let dx = mouse.x - self.wheeling_ref_mouse.x;
            let dy = mouse.y - self.wheeling_ref_mouse.y;
            let threshold = self.io.mouse_drag_threshold;
            if dx * dx + dy * dy > threshold * threshold {
                self.wheeling_release_timer = 0.0;
            }
        }
        if self.wheeling_release_timer <= 0.0 {
            self.wheeling_window = None;
        }
    }

    /// `g.Windows` order, back to front: the order the lists are drawn in.
    pub fn display_order(&self) -> &[String] {
        &self.display_order
    }

    /// `ImGui::Render` + `ImGui::GetDrawData`: the draw lists of the frame in display order.
    ///
    /// A window that drew nothing is left out, exactly like `AddDrawListToDrawDataEx` does.
    pub fn render_lists(&self) -> Vec<(String, crate::draw_list::DrawList)> {
        let mut out = Vec::new();
        for name in &self.display_order {
            let Some((_, list)) = self.frame_lists.iter().find(|(known, _)| known == name) else {
                continue;
            };
            if list.is_empty() {
                continue;
            }
            out.push((name.clone(), list.clone()));
        }
        out
    }

    /// `ImGui::UpdateMouseMovingWindowNewFrame`: while `g.MovingWindow` is set and the mouse is
    /// down, the window's position becomes the mouse minus where inside it the drag started.
    fn update_mouse_moving_window_new_frame(&mut self) {
        let Some(name) = self.moving_window.clone() else {
            // `_NoMove` windows still take the move id, to stop the click hovering other windows;
            // it is released with the button.
            if self.active_id == Some(self.get_id("#MOVE")) && !self.io.mouse_down[0] {
                self.active_id = None;
            }
            return;
        };
        if !self.io.mouse_down[0] || self.io.mouse_pos.is_none() {
            // `StopMouseMovingWindow()` plus the reference's `ClearActiveID()`.
            self.moving_window = None;
            self.active_id = None;
            return;
        }
        let mouse = self.io.mouse_pos.unwrap_or_default();
        let pos = Vec2 {
            x: mouse.x - self.active_click_offset.x,
            y: mouse.y - self.active_click_offset.y,
        };
        // `SetWindowPos(..., ImGuiCond_Always)`.
        if let Some(entry) = WindowMemory::find_mut(&mut self.memory, &name) {
            entry.pos = pos;
        }
    }

    /// `ImGui::SetNextWindowScroll`: the scroll the next window starts with. It is clamped to the
    /// maximum the window computes, which lags one frame behind the content.
    pub fn set_next_window_scroll(&mut self, scroll: Vec2) {
        self.next_scroll = Some(scroll);
    }

    /// The `ImGuiWindowFlags` of the next window, as `ImGui::Begin` takes them.
    /// ImGui::SetMouseCursor: what the backend should show for the rest of the frame.
    pub fn set_mouse_cursor(&mut self, cursor: crate::io::MouseCursor) {
        self.io.mouse_cursor = cursor;
    }

    pub fn set_next_window_flags(&mut self, flags: WindowFlags) {
        self.next_flags = Some(flags);
    }

    /// Pins `ImDrawListSharedData::TexUvLines`: the atlas line-texture table a stroked primitive
    /// samples when the draw list has anti-aliased lines enabled.
    pub fn set_line_uv_lines(&mut self, table: Vec<[f32; 4]>) {
        self.line_uv_lines = table;
    }

    /// `ImGuiIO` for reading the computed state.
    pub fn io(&self) -> &Io {
        &self.io
    }

    /// `ImGuiIO` for feeding raw input before the next `begin_frame`.
    pub fn io_mut(&mut self) -> &mut Io {
        &mut self.io
    }

    /// `g.Time`: seconds accumulated from `DeltaTime`, kept as the reference's `double`.
    /// Replaces the font used for drawing, e.g. with one built from a pinned glyph table.
    pub fn set_font_for_drawing(&mut self, font: crate::font::Font) {
        self.font = font;
    }

    /// The draw list of the open window, for adding geometry by hand.
    pub fn draw_list_mut(&mut self) -> &mut crate::draw_list::DrawList {
        &mut self.draw_list
    }

    /// The draw list of the open window, or of the last window that closed in this frame.
    pub fn draw_list(&self) -> &crate::draw_list::DrawList {
        if self.windows.is_empty() {
            &self.last_list
        } else {
            &self.draw_list
        }
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    /// The reference's `UpdateInputEvents` (key events only) followed by `UpdateKeyboardInputs`:
    /// queued events are applied, then durations and the modifier flags are derived.
    fn update_keyboard_inputs(&mut self) {
        let queued = core::mem::take(&mut self.io.key_queue);
        for (key, down) in queued {
            let data = &mut self.io.keys[key.index()];
            data.down = down;
            data.analog_value = if down { 1.0 } else { 0.0 };
        }

        let delta_time = self.io.delta_time;
        for data in &mut self.io.keys {
            data.down_duration = if data.down {
                if data.down_duration < 0.0 {
                    0.0
                } else {
                    data.down_duration + delta_time
                }
            } else {
                -1.0
            };
        }

        let keys = &self.io.keys;
        self.io.key_ctrl = keys[Key::ModCtrl.index()].down;
        self.io.key_shift = keys[Key::ModShift.index()].down;
        self.io.key_alt = keys[Key::ModAlt.index()].down;
        self.io.key_super = keys[Key::ModSuper.index()].down;
    }

    /// The reference's `UpdateMouseInputs`, restricted to position, buttons and double clicks.
    fn update_mouse_inputs(&mut self) {
        let time = self.time;
        let delta_time = self.io.delta_time;

        // The reference rounds the position to avoid spreading sub-pixel values.
        if let Some(pos) = self.io.mouse_pos {
            self.io.mouse_pos = Some(Vec2 {
                x: pos.x.floor(),
                y: pos.y.floor(),
            });
        }

        self.io.mouse_delta = match (self.io.mouse_pos, self.io.mouse_pos_prev) {
            (Some(current), Some(previous)) => Vec2 {
                x: current.x - previous.x,
                y: current.y - previous.y,
            },
            _ => Vec2::default(),
        };

        for i in 0..BUTTONS {
            self.io.mouse_clicked[i] =
                self.io.mouse_down[i] && self.io.mouse_down_duration[i] < 0.0;
            self.io.mouse_released[i] =
                !self.io.mouse_down[i] && self.io.mouse_down_duration[i] >= 0.0;
            self.io.mouse_clicked_count[i] = 0;

            let duration = self.io.mouse_down_duration[i];
            self.io.mouse_down_duration[i] = if self.io.mouse_down[i] {
                if duration < 0.0 {
                    0.0
                } else {
                    duration + delta_time
                }
            } else {
                -1.0
            };

            if self.io.mouse_clicked[i] {
                let mut repeated = false;
                // The reference casts the difference to float before comparing, and both the
                // clock and the timeout are floats: keep that, it decides double clicks.
                let elapsed = (time - self.io.mouse_clicked_time[i]) as f32;
                if elapsed < self.io.mouse_double_click_time {
                    if let Some(pos) = self.io.mouse_pos {
                        let dx = pos.x - self.io.mouse_clicked_pos[i].x;
                        let dy = pos.y - self.io.mouse_clicked_pos[i].y;
                        let max = self.io.mouse_double_click_max_dist;
                        if dx * dx + dy * dy < max * max {
                            repeated = true;
                        }
                    }
                }
                self.io.mouse_clicked_last_count[i] = if repeated {
                    self.io.mouse_clicked_last_count[i] + 1
                } else {
                    1
                };
                self.io.mouse_clicked_time[i] = time;
                self.io.mouse_clicked_pos[i] = self.io.mouse_pos.unwrap_or_default();
                self.io.mouse_clicked_count[i] = self.io.mouse_clicked_last_count[i];
                // A fresh press starts the drag distance over.
                self.io.mouse_drag_max_distance_sqr[i] = 0.0;
            } else if self.io.mouse_down[i] {
                // Keep the furthest distance from the press: this is what turns a held button
                // into a drag once it passes `MouseDragThreshold`.
                let delta_sqr = self
                    .io
                    .mouse_pos
                    .map(|pos| {
                        let dx = pos.x - self.io.mouse_clicked_pos[i].x;
                        let dy = pos.y - self.io.mouse_clicked_pos[i].y;
                        dx * dx + dy * dy
                    })
                    .unwrap_or(0.0);
                self.io.mouse_drag_max_distance_sqr[i] =
                    self.io.mouse_drag_max_distance_sqr[i].max(delta_sqr);
            }
        }

        self.io.mouse_pos_prev = self.io.mouse_pos;
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn last_frame_draw_calls(&self) -> usize {
        self.last_frame_draw_calls
    }

    /// Identifier of the current scope, `0` at the root like the reference's window-less case.
    pub fn current_id(&self) -> ImGuiID {
        self.id_stack.last().copied().unwrap_or(0)
    }

    /// Hash of `label` in the current scope (`ImGui::GetID`).
    pub fn get_id(&self, label: &str) -> ImGuiID {
        hash_str_utf8(label, self.current_id())
    }

    /// Opens a scope seeded by `label` (`ImGui::PushID`).
    pub fn push_id(&mut self, label: &str) {
        let id = self.get_id(label);
        self.id_stack.push(id);
    }

    /// Closes the innermost scope (`ImGui::PopID`).
    pub fn pop_id(&mut self) {
        self.id_stack.pop();
    }

    pub fn id_stack_depth(&self) -> usize {
        self.id_stack.len()
    }

    /// A fresh draw list with the flags and atlas tables a window's list starts with.
    fn new_window_draw_list(&self) -> crate::draw_list::DrawList {
        let mut draw_list = crate::draw_list::DrawList::new();
        draw_list.flags = crate::draw_list::WINDOW_DRAW_FLAGS;
        // `_FringeScale` at pixel density 1.
        draw_list.fringe_scale = 1.0;
        draw_list.tex_uv_lines.clone_from(&self.line_uv_lines);
        draw_list
    }

    /// `ImGui::Begin` with `SetNextWindowPos`/`SetNextWindowSize` already applied.
    ///
    /// `None` keeps the position or size the window had in a previous frame, as the reference does.
    /// Starts a window and paints its background, like `ImGui::Begin` with default flags.
    ///
    /// Returns what the reference returns: `false` once the window is collapsed, so callers skip
    /// its contents.
    pub fn begin_window(&mut self, name: &str, pos: Option<Vec2>, size: Option<Vec2>) -> bool {
        self.begin_window_impl(
            name,
            pos,
            size,
            WindowOptions {
                title: None,
                background: true,
                open: None,
                collapsible: false,
                flags: WindowFlags::NO_MOVE | WindowFlags::NO_SCROLLBAR | WindowFlags::NO_RESIZE,
            },
        )
    }

    /// Starts a window with a title bar: background, title band, border and the title text.
    pub fn begin_window_titled(
        &mut self,
        name: &str,
        title: &str,
        pos: Option<Vec2>,
        size: Option<Vec2>,
    ) -> bool {
        self.begin_window_impl(
            name,
            pos,
            size,
            WindowOptions {
                title: Some(title),
                background: true,
                open: None,
                collapsible: false,
                flags: WindowFlags::NO_MOVE | WindowFlags::NO_SCROLLBAR | WindowFlags::NO_RESIZE,
            },
        )
    }

    /// Starts a titled window that can be moved: the reference's default, without
    /// `ImGuiWindowFlags_NoMove`. A click on it starts a move drag.
    pub fn begin_window_titled_movable(
        &mut self,
        name: &str,
        title: &str,
        pos: Option<Vec2>,
        size: Option<Vec2>,
    ) -> bool {
        self.begin_window_impl(
            name,
            pos,
            size,
            WindowOptions {
                title: Some(title),
                background: true,
                open: None,
                collapsible: false,
                flags: WindowFlags::NO_SCROLLBAR | WindowFlags::NO_RESIZE,
            },
        )
    }

    /// Starts a titled window that also carries the window icons: a collapse arrow on the title
    /// bar's left and, when `open` is `Some`, a close cross on its right.
    ///
    /// This mirrors `ImGui::Begin("W", &open, flags)` with neither `NoCollapse` nor a null
    /// `p_open`. The icons shift the title text, so the bar is laid out the way the reference's
    /// `RenderWindowTitleBarContents` does. Only `Style::WindowMenuButtonPosition == Left`, the
    /// default, is modelled.
    ///
    /// The arrow toggles the collapsed state from the next frame on; the cross sets `*open` to
    /// `false` when it is released inside. Returns `false` while the window is collapsed.
    ///
    /// Not modelled yet: dragging the title bar itself (only the arrow drag works, which is the
    /// reference's `CollapseButton` path).
    pub fn begin_window_titled_with_icons(
        &mut self,
        name: &str,
        title: &str,
        pos: Option<Vec2>,
        size: Option<Vec2>,
        open: Option<&mut bool>,
        no_move: bool,
    ) -> bool {
        self.begin_window_impl(
            name,
            pos,
            size,
            WindowOptions {
                title: Some(title),
                background: true,
                open,
                collapsible: true,
                flags: WindowFlags::NO_SCROLLBAR
                    | WindowFlags::NO_RESIZE
                    | if no_move {
                        WindowFlags::NO_MOVE
                    } else {
                        WindowFlags::NONE
                    },
            },
        )
    }

    /// Starts a window, choosing whether the background is painted
    /// (`ImGuiWindowFlags_NoBackground` when false).
    pub fn begin_window_ex(
        &mut self,
        name: &str,
        pos: Option<Vec2>,
        size: Option<Vec2>,
        background: bool,
    ) -> bool {
        self.begin_window_impl(
            name,
            pos,
            size,
            WindowOptions {
                title: None,
                background,
                open: None,
                collapsible: false,
                flags: WindowFlags::NO_MOVE | WindowFlags::NO_SCROLLBAR | WindowFlags::NO_RESIZE,
            },
        )
    }

    fn begin_window_impl(
        &mut self,
        name: &str,
        pos: Option<Vec2>,
        size: Option<Vec2>,
        opts: WindowOptions,
    ) -> bool {
        let title = opts.title;
        let background = opts.background;
        // `ImGui::Begin` takes the flags as an argument; the convenience constructors pass their
        // own, and `set_next_window_flags` overrides them for a window opened that way.
        let flags = self.next_flags.take().unwrap_or(opts.flags);
        let remembered = WindowMemory::find(&self.memory, name);
        // A window that has just appeared has no grips yet: the reference only handles them from
        // its second frame on.
        let window_just_created = remembered.is_none();
        let pos_was_set = pos.is_some();
        let (
            pos,
            size_full,
            mut collapsed,
            mut want_collapse_toggle,
            remembered_scroll,
            content,
            inner_before,
        ) = match remembered {
            Some(memory) => (
                pos.unwrap_or(memory.pos),
                size.unwrap_or(memory.size_full),
                memory.collapsed,
                memory.want_collapse_toggle,
                Some(memory.scroll),
                memory.content_size,
                memory.inner_size,
            ),
            None => (
                pos.unwrap_or_default(),
                size.unwrap_or_default(),
                false,
                false,
                None,
                Vec2::default(),
                Vec2::default(),
            ),
        };

        // Each window draws into its own list for the frame.
        self.draw_list = self.new_window_draw_list();
        // A window that has never been seen goes on top of the display order and takes the focus;
        // an existing one keeps its place until something raises it.
        if !self.display_order.iter().any(|known| known == name) {
            self.display_order.push(name.to_string());
            self.focused_window = Some(name.to_string());
        }

        // The arrow asks for the toggle; Begin applies it at the start of the next frame.
        if opts.collapsible && want_collapse_toggle {
            collapsed = !collapsed;
        }
        want_collapse_toggle = false;

        let pad = self.style.window_padding;
        let title_height = if title.is_some() {
            self.frame_height()
        } else {
            0.0
        };
        // `ImGuiWindowFlags_AlwaysAutoResize`: the window takes the size its content needs, measured
        // on the previous frame, plus its padding and the decorations above and below the content.
        // `CalcWindowAutoFitSize` does clamp to `WindowMinSize`, but the reference's first frame
        // shows the unclamped value here, and no scenario content is small enough to tell them
        // apart otherwise.
        let mut size_full = size_full;
        // `CalcWindowMinSize`: `WindowMinSize` for a normal window, the 1 pixel hard minimum for an
        // auto-sized one, which is why an auto-sized window can end up smaller than the style says.
        // A size *asked for* by API also wins over the auto-fit, as `!window_size_x_set_by_api`.
        let size_was_set = size.is_some();
        let auto_resize = flags.contains(WindowFlags::ALWAYS_AUTO_RESIZE) && !size_was_set;
        // `CalcWindowSizeAfterConstraint`.
        // The hard minimum depends on the flag alone, not on whether the size was asked for.
        let always_auto = flags.contains(WindowFlags::ALWAYS_AUTO_RESIZE);
        let hard_min = 1.0;
        size_full.x = size_full.x.max(if always_auto {
            hard_min
        } else {
            self.style.window_min_size.x
        });
        size_full.y = size_full.y.max(if always_auto {
            hard_min
        } else {
            self.style.window_min_size.y
        });
        if auto_resize {
            let decoration = title_height;
            size_full = Vec2 {
                x: (content.x + pad.x * 2.0).min(self.io.display_size.x),
                y: (content.y + pad.y * 2.0 + decoration).min(self.io.display_size.y),
            };
        }
        // `ClampWindowPos`: unless the position was asked for explicitly this frame, the window is
        // pushed back so at least part of it stays inside the work area.
        let clamp_size = if collapsed {
            Vec2 {
                x: size_full.x,
                y: title_height,
            }
        } else {
            size_full
        };
        let mut pos = if pos_was_set {
            pos
        } else {
            self.clamp_window_pos(pos, clamp_size)
        };
        // `UpdateWindowManualResize`: the input pass runs after the position clamp and before the
        // interior is computed, so a drag changes what the same frame draws with. The drawing
        // happens later, in the decoration pass, and only reads the colours this pass decided.
        if !flags.contains(WindowFlags::NO_RESIZE) && !window_just_created {
            if let Some((new_pos, new_size)) = self.update_resize(name, pos, size_full, always_auto)
            {
                pos = Vec2 {
                    x: new_pos.x.trunc(),
                    y: new_pos.y.trunc(),
                };
                size_full = new_size;
            }
        }
        // A collapsed window shrinks to its title bar: the reference keeps `SizeFull` and displays
        // `(Size.x, TitleBarHeight)`.
        let size = if collapsed {
            Vec2 {
                x: size_full.x,
                y: title_height,
            }
        } else {
            size_full
        };
        // `InnerRect`: the window rectangle without its decorations. A top-level window gives up
        // the title bar, and a vertical scrollbar its width on the right.
        let mut inner_rect = (
            Vec2 {
                x: pos.x,
                y: pos.y + title_height,
            },
            Vec2 {
                x: pos.x + size.x,
                y: pos.y + size.y,
            },
        );
        // `window->ScrollbarY`: the content of the previous frame did not fit.
        let wants_scrollbar = !flags.contains(WindowFlags::NO_SCROLLBAR);
        let wants_horizontal = wants_scrollbar && flags.contains(WindowFlags::HORIZONTAL_SCROLLBAR);
        // vail_size_from_last_frame: a window that just changed size decides with the size it had
        // before, which is why a tiny window can skip a scrollbar its content would need.
        let avail_before = if inner_before.x > 0.0 || inner_before.y > 0.0 {
            inner_before
        } else {
            Vec2 {
                x: inner_rect.1.x - inner_rect.0.x,
                y: inner_rect.1.y - inner_rect.0.y,
            }
        };
        let scrollbar_y = wants_scrollbar && content.y + pad.y * 2.0 > avail_before.y;
        if scrollbar_y {
            inner_rect.1.x -= self.style.scrollbar_size;
        }
        // `window->ScrollbarX`: the content did not fit horizontally either. It is decided after the
        // vertical one, so the interior is already narrower by that bar, and it gives up its height
        // on the bottom.
        let scrollbar_x =
            wants_scrollbar && wants_horizontal && content.x + pad.x * 2.0 > avail_before.x;
        if scrollbar_x {
            inner_rect.1.y -= self.style.scrollbar_size;
        }
        let scroll_max = Vec2 {
            x: (content.x + pad.x * 2.0 - (inner_rect.1.x - inner_rect.0.x)).max(0.0),
            y: (content.y + pad.y * 2.0 - (inner_rect.1.y - inner_rect.0.y)).max(0.0),
        };
        // `CalcNextScrollFromScrollTargetAndClamp`: the requested scroll, clamped to the maximum.
        // window->Scroll survives between frames; SetNextWindowScroll overrides it.
        let mut requested = self
            .next_scroll
            .take()
            .unwrap_or(remembered_scroll.unwrap_or_default());
        // `UpdateMouseWheel` + `SetScrollY`: the wheel scrolls the window it is locked to, falling
        // back to the hovered one. The step is five font sizes, capped at two thirds of the
        // interior.
        if self.io.mouse_wheel != 0.0 {
            let owner = self
                .wheeling_window
                .clone()
                .or_else(|| self.hovered_window.clone());
            if owner.as_deref() == Some(name) && scroll_max.y != 0.0 {
                // `LockWheelingWindow`: refresh the timer, and take the lock (with the mouse
                // position it was taken at) when it moves to this window.
                let amount = self.io.mouse_wheel.abs();
                self.wheeling_release_timer =
                    (self.wheeling_release_timer + amount * 0.70).min(0.70);
                if self.wheeling_window.as_deref() != Some(name) {
                    self.wheeling_window = Some(name.to_string());
                    self.wheeling_ref_mouse = self.io.mouse_pos.unwrap_or_default();
                }
                let max_step = (inner_rect.1.y - inner_rect.0.y) * 0.67;
                let scroll_step = (5.0 * self.font.size).min(max_step).trunc();
                requested.y -= self.io.mouse_wheel * scroll_step;
            }
        }
        let mut scroll = Vec2 {
            x: requested.x.clamp(0.0, scroll_max.x),
            y: requested.y.clamp(0.0, scroll_max.y),
        };
        // With a title bar the content starts below it, which is why the settled reference lays a
        // window out 19 pixels lower than its first frame does. The scroll moves it up.
        let mut content_origin = Vec2 {
            x: inner_rect.0.x + pad.x - scroll.x,
            y: inner_rect.0.y + pad.y - scroll.y,
        };

        // The background is painted before the window clip is applied, with the fullscreen clip.
        self.draw_list
            .set_current_clip(self.draw_list.fullscreen_clip);
        if collapsed {
            // A collapsed window draws its title bar as a frame and nothing else: no background,
            // no band, no outer border, no resize grips.
            let bar_max = Vec2 {
                x: pos.x + size.x,
                y: pos.y + title_height,
            };
            self.draw_list.prim_rect(
                pos,
                bar_max,
                self.style.color_u32(crate::style::Col::TitleBgCollapsed),
            );
            // `RenderFrame` outlines the bar with `Style::FrameBorderSize`, which the reference
            // temporarily sets to the window border size. The border shadow it draws first has a
            // transparent colour, so the reference's `AddRect` drops it.
            if self.style.window_border_size > 0.0 {
                self.draw_list.add_aligned_rect_border(
                    pos,
                    bar_max,
                    self.style.color_u32(crate::style::Col::Border),
                    self.style.window_border_size,
                );
            }
        } else if background {
            // With a title bar the background starts below it: the band covers the top.
            self.draw_list.prim_rect(
                Vec2 {
                    x: pos.x,
                    y: pos.y + title_height,
                },
                Vec2 {
                    x: pos.x + size.x,
                    y: pos.y + size.y,
                },
                self.style.color_u32(crate::style::Col::WindowBg),
            );
        }

        if !collapsed {
            // The title band, between the background and the border.
            if title.is_some() {
                self.draw_list.prim_rect(
                    pos,
                    Vec2 {
                        x: pos.x + size.x,
                        y: pos.y + title_height,
                    },
                    self.title_bar_color(name),
                );
            }

            let outer = (
                pos,
                Vec2 {
                    x: pos.x + size.x,
                    y: pos.y + size.y,
                },
            );
            // The scrollbars go between the title band and the window border, as the reference's
            // decoration pass does. The horizontal one is drawn first, then the vertical.
            if scrollbar_x {
                if let Some(seeked) =
                    self.draw_scrollbar(name, outer, inner_rect, content, scroll, true)
                {
                    scroll = seeked;
                    content_origin = Vec2 {
                        x: inner_rect.0.x + pad.x - scroll.x,
                        y: inner_rect.0.y + pad.y - scroll.y,
                    };
                }
            }
            if scrollbar_y {
                if let Some(seeked) =
                    self.draw_scrollbar(name, outer, inner_rect, content, scroll, false)
                {
                    scroll = seeked;
                    content_origin = Vec2 {
                        x: inner_rect.0.x + pad.x - scroll.x,
                        y: inner_rect.0.y + pad.y - scroll.y,
                    };
                }
            }

            // `RenderResizeGrips`: between the scrollbars and the border, so the border draws over
            // them and they never sit on top of the frame.
            if !flags.contains(WindowFlags::NO_RESIZE) && !window_just_created {
                self.draw_resize_grips(pos, size);
            }

            // The window border, inside the rectangle. A pixel-aligned window takes the two-ring
            // frame; fractional coordinates would take the miter stroke, not wired up here.
            // The reference draws the border only when the window has a background.
            if background && self.style.window_border_size > 0.0 {
                self.draw_list.add_aligned_rect_border(
                    pos,
                    Vec2 {
                        x: pos.x + size.x,
                        y: pos.y + size.y,
                    },
                    self.style.color_u32(crate::style::Col::Border),
                    self.style.window_border_size,
                );
            }
        }

        // The title bar's contents: the icons first, then the text. This is the reference's
        // `RenderWindowTitleBarContents`, and the order matters because it changes the vertex
        // order of the list.
        if let Some(title) = title {
            let (pressed_collapse, pressed_close) = self.render_title_bar_contents(
                name,
                title,
                pos,
                size,
                opts.open.is_some(),
                opts.collapsible,
                collapsed,
                !opts.flags.contains(WindowFlags::NO_MOVE),
            );
            if pressed_collapse {
                want_collapse_toggle = true;
            }
            if pressed_close {
                if let Some(open) = opts.open {
                    *open = false;
                }
            }
        }

        // The window's draw list is clipped to its rectangle, inset by the window border.
        // The clip starts below the title band when there is one. A collapsed window gets an
        // empty clip here, which is what the reference gets too.
        let clip_top = self.style.window_border_size.max(title_height);
        self.draw_list.push_clip_rect(
            Vec2 {
                x: pos.x + self.style.window_border_size,
                y: pos.y + clip_top,
            },
            Vec2 {
                x: (inner_rect.1.x - self.style.window_border_size * 0.5).floor(),
                y: (inner_rect.1.y - self.style.window_border_size * 0.5).floor(),
            },
            false,
        );
        let content_size = Vec2 {
            x: size.x - pad.x * 2.0,
            y: size.y - pad.y * 2.0,
        };
        self.windows.push(Window {
            name: name.to_string(),
            pos,
            size,
            size_full,
            collapsed,
            want_collapse_toggle,
            flags,
            scroll,
            scroll_max,
            content_extent: content,
            scrollbar_y,
            scrollbar_x,
            inner_rect,
            content_origin,
            content_size,
            cursor: content_origin,
            cursor_start: content_origin,
            cursor_max: content_origin,
            line_start_x: content_origin.x,
            line_height: 0.0,
            last_min: Vec2::default(),
            last_max: Vec2::default(),
            has_item: false,
        });

        // `Begin` reports false while the window is collapsed, so callers skip its contents.
        !collapsed
    }

    /// `ImGui::RenderWindowTitleBarContents`: the collapse arrow, the close cross and the title,
    /// laid out so the buttons never overlap the text.
    #[allow(clippy::too_many_arguments)]
    fn render_title_bar_contents(
        &mut self,
        name: &str,
        title: &str,
        pos: Vec2,
        size: Vec2,
        closable: bool,
        collapsible: bool,
        collapsed: bool,
        can_move: bool,
    ) -> (bool, bool) {
        use crate::style::Col;

        let border = self.style.window_border_size;
        let frame_padding = self.style.frame_padding;
        let inner = self.style.item_inner_spacing.x;
        let button_sz = self.font.size;
        let title_height = self.frame_height();

        // The caller insets the bar by the window border on both sides but not vertically.
        let bar_min_x = pos.x + border;
        let bar_max_x = pos.x + size.x - border;
        let button_y = pos.y + frame_padding.y;

        let mut pad_l = frame_padding.x;
        let mut pad_r = frame_padding.x;

        let close_pos = if closable {
            let p = Vec2 {
                x: bar_max_x - pad_r - button_sz,
                y: button_y,
            };
            pad_r += button_sz + inner;
            Some(p)
        } else {
            None
        };
        // `Style::WindowMenuButtonPosition` defaults to Left, which is the side modelled here.
        let collapse_pos = if collapsible {
            let p = Vec2 {
                x: bar_min_x + pad_l,
                y: button_y,
            };
            pad_l += button_sz + inner;
            Some(p)
        } else {
            None
        };

        // Keep centred titles centred whether or not the buttons are there.
        if pad_l > frame_padding.x {
            pad_l += inner;
        }
        if pad_r > frame_padding.x {
            pad_r += inner;
        }

        let text_color = self.style.color_u32(Col::Text);
        let mut pressed_collapse = false;
        let mut pressed_close = false;

        if let Some(min) = collapse_pos {
            let collapse_id = self.get_id("#COLLAPSE");
            let (pressed, hovered, held) = self.icon_button(collapse_id, min, button_sz, pos, size);
            pressed_collapse = pressed;
            // `(held && hovered) ? ButtonActive : hovered ? ButtonHovered : Button`, and the
            // background is drawn whenever the button is hovered or held.
            if hovered || held {
                let colour = if held && hovered {
                    Col::ButtonActive
                } else if hovered {
                    Col::ButtonHovered
                } else {
                    Col::Button
                };
                self.draw_list.add_rect_filled(
                    min,
                    Vec2 {
                        x: min.x + button_sz,
                        y: min.y + button_sz,
                    },
                    self.style.color_u32(colour),
                );
            }
            // Down while the window is open, right once it is collapsed.
            let (cos_a, sin_a) = if collapsed {
                (ARROW_RIGHT_COS, ARROW_RIGHT_SIN)
            } else {
                (ARROW_DOWN_COS, ARROW_DOWN_SIN)
            };
            self.draw_list.add_arrow(
                min,
                text_color,
                cos_a,
                sin_a,
                1.0,
                self.font.size,
                self.style.symbol_arrow_size_factor,
                self.style.symbol_arrow_thickness_factor,
            );

            // Dragging past the threshold while the arrow is active moves the window instead.
            if self.active_id == Some(collapse_id) && self.io.is_mouse_dragging(0, -1.0) {
                self.start_mouse_moving_window(name, pos, can_move);
            }
        }

        if let Some(min) = close_pos {
            let (pressed, hovered, held) =
                self.icon_button(self.get_id("#CLOSE"), min, button_sz, pos, size);
            pressed_close = pressed;
            // The reference chooses the colour from `held` alone but only fills while hovered.
            if hovered {
                let colour = if held {
                    Col::ButtonActive
                } else {
                    Col::ButtonHovered
                };
                self.draw_list.add_rect_filled(
                    min,
                    Vec2 {
                        x: min.x + button_sz,
                        y: min.y + button_sz,
                    },
                    self.style.color_u32(colour),
                );
            }
            let center = Vec2 {
                x: min.x + button_sz * 0.5,
                y: min.y + button_sz * 0.5,
            };
            let half = button_sz * 0.5;
            let extent = half * crate::draw_list::REF_0_7071 - 1.0;
            let thickness = 1.0; // `1.0f * Style._MainScale`, which is 1 at density 1.
            self.draw_list.add_line(
                Vec2 {
                    x: center.x + extent,
                    y: center.y + extent,
                },
                Vec2 {
                    x: center.x - extent,
                    y: center.y - extent,
                },
                text_color,
                thickness,
                0,
            );
            self.draw_list.add_line(
                Vec2 {
                    x: center.x + extent,
                    y: center.y - extent,
                },
                Vec2 {
                    x: center.x - extent,
                    y: center.y + extent,
                },
                text_color,
                thickness,
                0,
            );
        }

        // The title, inside the space the buttons leave.
        let text_size = self.font.calc_text_size(title, self.font.size);
        // The reference clips the title to this rectangle before drawing it. A title long enough
        // to be cut is not covered by a golden yet, so only the left edge is used.
        let _clip_max_x = (bar_max_x - pad_r + inner).min(bar_max_x);
        let font = self.font.clone();
        self.draw_list.add_text(
            &font,
            font.size,
            Vec2 {
                x: bar_min_x + pad_l,
                y: pos.y + (title_height - text_size.y) * self.style.window_title_align.y,
            },
            text_color,
            title,
        );

        (pressed_collapse, pressed_close)
    }

    /// `ButtonBehavior` for one icon: hover, hold and press, with the reference's rule that the
    /// press only counts when the button is released inside.
    ///
    /// The icons do not call `ItemSize`, so unlike every other widget they leave the cursor alone.
    /// The window rectangle is passed in because the window is not on the stack yet while its
    /// title bar is being drawn.
    fn icon_button(
        &mut self,
        id: ImGuiID,
        min: Vec2,
        button_sz: f32,
        window_pos: Vec2,
        window_size: Vec2,
    ) -> (bool, bool, bool) {
        // `ItemHoverable`: while another item owns the mouse, nothing else is hovered.
        let hovered = self.icon_hovered(min, button_sz, window_pos, window_size)
            && (self.active_id.is_none() || self.active_id == Some(id));

        if hovered {
            self.hovered_id = Some(id);
        }
        if hovered && self.io.mouse_clicked[0] && self.active_id != Some(id) {
            self.active_id = Some(id);
        }

        let mut pressed = false;
        let mut held = false;
        if self.active_id == Some(id) {
            if self.io.mouse_down[0] {
                held = true;
            } else {
                // Releasing inside presses; releasing anywhere clears the active id.
                pressed = hovered;
                self.active_id = None;
            }
        }
        (pressed, hovered, held)
    }

    /// `ImGui::StartMouseMovingWindow`: the drag takes over the active id, remembers where inside
    /// the window it started, and (when the window may move) makes it follow the mouse.
    ///
    /// The reference also focuses the window and claims the keyboard; there is no focus stack or
    /// keyboard model here yet, so neither is done.
    fn start_mouse_moving_window(&mut self, name: &str, window_pos: Vec2, can_move: bool) {
        // `window->MoveId`. The reference seeds it with the window id; here the identifier stack is
        // not window-scoped yet, so it is a plain hash. What matters for the port is that it is a
        // different id from the icons', which is what cancels their press.
        self.active_id = Some(self.get_id("#MOVE"));
        self.active_click_offset = Vec2 {
            x: self.io.mouse_clicked_pos[0].x - window_pos.x,
            y: self.io.mouse_clicked_pos[0].y - window_pos.y,
        };
        if can_move {
            self.moving_window = Some(name.to_string());
        }
    }

    /// Whether the mouse is over an icon: inside the window's rectangle and inside its square.
    fn icon_hovered(&self, min: Vec2, button_sz: f32, window_pos: Vec2, window_size: Vec2) -> bool {
        let Some(mouse) = self.io.mouse_pos else {
            return false;
        };
        let inside_window = mouse.x >= window_pos.x
            && mouse.x < window_pos.x + window_size.x
            && mouse.y >= window_pos.y
            && mouse.y < window_pos.y + window_size.y;
        inside_window
            && mouse.x >= min.x
            && mouse.x < min.x + button_sz
            && mouse.y >= min.y
            && mouse.y < min.y + button_sz
    }

    /// `ImGui::End`: the window remembers its position, its full size and its collapsed state.
    pub fn end_window(&mut self) {
        self.draw_list.finish_clip_scope();
        if let Some(window) = self.windows.pop() {
            // `CalcWindowContentSizes`: the extent the layout reached, measured from the scrolled
            // start so scrolling does not change it.
            let content_size = Vec2 {
                x: (window.cursor_max.x - window.cursor_start.x).trunc(),
                y: (window.cursor_max.y - window.cursor_start.y).trunc(),
            };
            let entry = WindowMemory {
                name: window.name.clone(),
                pos: window.pos,
                size_full: window.size_full,
                collapsed: window.collapsed,
                want_collapse_toggle: window.want_collapse_toggle,
                scroll: window.scroll,
                content_size,
                inner_size: Vec2 {
                    x: window.size.x,
                    y: (window.size.y
                        - (window.size.y - window.inner_rect.1.y + window.inner_rect.0.y))
                        .max(0.0),
                },
                flags: window.flags,
            };
            match WindowMemory::find_mut(&mut self.memory, &window.name) {
                Some(slot) => *slot = entry,
                None => self.memory.push(entry),
            }
            // The frame's list for this window, kept for `render_lists`.
            let list = core::mem::take(&mut self.draw_list);
            self.frame_lists.push((window.name.clone(), list.clone()));
            self.last_list = list;
        }
    }

    /// `ImGui::EndFrame`: what the reference does once every window has been submitted.
    ///
    /// A left click on a window's empty space raises it. The raise happens here, before the frame
    /// is rendered, which is why the click's own frame already draws the window on top.
    pub fn end_frame(&mut self) {
        // `WasActive` for the next frame's hover search.
        self.last_frame_windows = self
            .frame_lists
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        // `UpdateMouseMovingWindowEndFrame` bails out while an item owns the mouse.
        if self.active_id.is_some() || self.hovered_id.is_some() {
            return;
        }
        if !self.io.mouse_clicked[0] {
            return;
        }
        let Some(hovered) = self.hovered_window.clone() else {
            return;
        };
        // `FocusWindow`: the window moves to the end of the display order.
        if let Some(index) = self.display_order.iter().position(|name| *name == hovered) {
            let name = self.display_order.remove(index);
            self.display_order.push(name);
        }
        // `StartMouseMovingWindow`: the same click also starts a move drag. The window only
        // follows the mouse when it is not `NoMove`, but the active id is taken either way.
        if let Some(memory) = WindowMemory::find(&self.memory, &hovered) {
            let pos = memory.pos;
            let can_move = !memory.flags.contains(WindowFlags::NO_MOVE);
            let hovered = hovered.clone();
            self.start_mouse_moving_window(&hovered, pos, can_move);
        }
        self.focused_window = Some(hovered);
    }

    /// `ImGui::Scrollbar` for the vertical axis: draws the bar and, while the grab is held, seeks
    /// the scroll. Returns the new scroll when holding changed it.
    ///
    /// Only the absolute-seek mode is modelled: `ConfigScrollbarScrollByPage` is off by default, so
    /// a click anywhere goes to the clicked location and holding the grab keeps the offset it was
    /// grabbed with.
    fn draw_scrollbar(
        &mut self,
        name: &str,
        outer: (Vec2, Vec2),
        inner_rect: (Vec2, Vec2),
        content: Vec2,
        scroll: Vec2,
        horizontal: bool,
    ) -> Option<Vec2> {
        use crate::style::Col;

        let axis = if horizontal { 0 } else { 1 };
        let other = 1 - axis;
        let get = |v: Vec2| if axis == 0 { v.x } else { v.y };
        let geto = |v: Vec2| if other == 0 { v.x } else { v.y };

        let border = self.style.window_border_size;
        // `GetWindowScrollbarRect`: a strip along one edge, between the window's inner edges.
        let mut frame_min = Vec2::default();
        let mut frame_max = Vec2::default();
        let (vmin, vmax) = if horizontal {
            (inner_rect.0.x + border, inner_rect.1.x - border)
        } else {
            (
                outer
                    .0
                    .x
                    .max(outer.1.x - border - self.style.scrollbar_size),
                outer.1.x - border,
            )
        };
        let (omin, omax) = if horizontal {
            (
                outer.1.y - border - self.style.scrollbar_size,
                outer.1.y - border,
            )
        } else {
            (inner_rect.0.y + border, inner_rect.1.y - border)
        };
        frame_min.x = vmin;
        frame_max.x = vmax;
        frame_min.y = omin;
        frame_max.y = omax;
        let frame_size_v = get(frame_max) - get(frame_min);
        let frame_size_o = geto(frame_max) - geto(frame_min);
        if frame_size_v <= 0.0 || frame_size_o <= 0.0 {
            return None;
        }
        // A vertical bar squeezed thinner than it is wide fades out, and stops taking clicks
        // while it is faded.
        let alpha = if !horizontal && frame_size_v < frame_size_o {
            (frame_size_v / (frame_size_o * 2.0).max(1.0)).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let padding = self
            .style
            .scrollbar_padding
            .min(frame_size_v.min(frame_size_o) * 0.5)
            .trunc();
        let mut bar_min = frame_min;
        let mut bar_max = frame_max;
        bar_min.x += padding;
        bar_max.x -= padding;
        bar_min.y += padding;
        bar_max.y -= padding;
        let scrollbar_size_v = get(bar_max) - get(bar_min);
        if scrollbar_size_v < 1.0 {
            return None;
        }
        let visible = get(inner_rect.1) - get(inner_rect.0);
        let contents = get(content) + get(self.style.window_padding) * 2.0;
        let win_size_v = contents.max(visible).max(1.0);
        // ImMin(bb.GetSize()[axis], style.GrabMinSize): the minimum is measured *along* the bar,
        // not across it, which only matters once the clamp engages (a very small window).
        let grab_min = scrollbar_size_v.min(self.style.grab_min_size);
        // `(float)(int)ImClamp(...)`: the reference truncates the grab to whole pixels.
        let grab_h_pixels = (scrollbar_size_v * (visible / win_size_v))
            .clamp(grab_min, scrollbar_size_v)
            .trunc();
        let grab_h_norm = grab_h_pixels / scrollbar_size_v;

        // `ButtonBehavior` over the bar's frame: the bar is one item of the window.
        let id = self.get_id("##scrollbar");
        let mouse = self.io.mouse_pos.unwrap_or_default();
        let inside = mouse.x >= frame_min.x
            && mouse.x < frame_max.x
            && mouse.y >= frame_min.y
            && mouse.y < frame_max.y;
        let own_window = self.hovered_window.as_deref() == Some(name);
        let hovered =
            own_window && inside && (self.active_id.is_none() || self.active_id == Some(id));
        if hovered {
            // An item owns the mouse: this also stops a click on it from moving the window.
            self.hovered_id = Some(id);
        }
        if hovered && self.io.mouse_clicked[0] {
            self.active_id = Some(id);
            self.active_id_just_activated = true;
        }
        let mut held = self.active_id == Some(id) && self.io.mouse_down[0];
        if self.active_id == Some(id) && !self.io.mouse_down[0] {
            // Release: the reference clears the active id and reports a press instead.
            self.active_id = None;
            held = false;
        }

        // `scroll_max` as an integer, like the reference's `ImS64`.
        let scroll_max = (contents - visible).max(1.0).trunc();
        let mut grab_v_norm = (get(scroll) / scroll_max).clamp(0.0, 1.0)
            * (scrollbar_size_v - grab_h_pixels)
            / scrollbar_size_v;
        let mut new_scroll = None;

        if held && alpha >= 1.0 && grab_h_norm < 1.0 {
            let mouse_v = if horizontal { mouse.x } else { mouse.y };
            let clicked_v_norm = ((mouse_v - get(bar_min)) / scrollbar_size_v).clamp(0.0, 1.0);
            let held_dir = if clicked_v_norm < grab_v_norm {
                -1.0
            } else if clicked_v_norm > grab_v_norm + grab_h_norm {
                1.0
            } else {
                0.0
            };
            if self.active_id_just_activated {
                // `ConfigScrollbarScrollByPage` off means a click always goes to where it landed;
                // with it on, a click outside the grab scrolls a page, and Shift forces that too.
                let seek_to_click = !self.io.config_scrollbar_scroll_by_page
                    || self.io.key_shift
                    || held_dir == 0.0;
                self.scrollbar_seek_mode = if seek_to_click { 0 } else { held_dir as i32 };
                self.scrollbar_click_delta = if held_dir == 0.0 && !self.io.key_shift {
                    clicked_v_norm - grab_v_norm - grab_h_norm * 0.5
                } else {
                    0.0
                };
            }
            let seeked = if self.scrollbar_seek_mode == 0 {
                let scroll_v_norm =
                    ((clicked_v_norm - self.scrollbar_click_delta - grab_h_norm * 0.5)
                        / (1.0 - grab_h_norm))
                        .clamp(0.0, 1.0);
                (scroll_v_norm * scroll_max).trunc()
            } else {
                // Page by page: a page is the visible size. The reference repeats while the button
                // is held; key repeat is not modelled, so this is the click that started the press.
                let page_dir = self.scrollbar_seek_mode as f32;
                (get(scroll) + page_dir * visible).clamp(0.0, scroll_max)
            };
            if seeked != get(scroll) {
                let mut value = scroll;
                if horizontal {
                    value.x = seeked;
                } else {
                    value.y = seeked;
                }
                new_scroll = Some(value);
            }
            let ratio = (seeked / scroll_max).clamp(0.0, 1.0);
            grab_v_norm = ratio * (scrollbar_size_v - grab_h_pixels) / scrollbar_size_v;
        }
        self.active_id_just_activated = false;

        let grab_col = if held {
            self.style.color_u32(Col::ScrollbarGrabActive)
        } else if hovered {
            self.style.color_u32(Col::ScrollbarGrabHovered)
        } else {
            self.style.color_u32(Col::ScrollbarGrab)
        };
        // `GetColorU32(col, alpha)`: the alpha argument scales the colour's own alpha.
        let scale = |color: u32| {
            let a = ((color >> 24) & 0xFF) as f32;
            let a = (a * alpha) as u32;
            (color & 0x00FF_FFFF) | (a << 24)
        };
        let grab_col = scale(grab_col);
        let bg_col = scale(self.style.color_u32(Col::ScrollbarBg));
        self.draw_list.prim_rect(frame_min, frame_max, bg_col);
        let grab_start = get(bar_min) + (get(bar_max) - get(bar_min)) * grab_v_norm;
        let mut grab_min = bar_min;
        let mut grab_max = bar_max;
        if horizontal {
            grab_min.x = grab_start;
            grab_max.x = grab_start + grab_h_pixels;
        } else {
            grab_min.y = grab_start;
            grab_max.y = grab_start + grab_h_pixels;
        }
        self.draw_list.add_rect_filled_aa_rounded(
            grab_min,
            grab_max,
            self.style.scrollbar_rounding,
            grab_col,
        );
        new_scroll
    }

    /// The work area is the whole display here, inset by `max(DisplayWindowPadding,
    /// DisplaySafeAreaPadding)` on each side.
    /// `UpdateWindowManualResize` for the two lower grips: hover, press, and while the lower-right
    /// one is held it resizes the window. The colour it decides is drawn later, in the decoration
    /// pass, so a drag has no frame of latency.
    /// `CalcResizePosSizeFromAnyCorner`: from the corner the mouse asks for, the window's position
    /// and the size it would need. The other corner stays where it is, and the min size constrains.
    fn resize_from_corner(
        &self,
        pos: Vec2,
        size: Vec2,
        corner_target: Vec2,
        corner_norm: Vec2,
        always_auto: bool,
    ) -> (Vec2, Vec2) {
        let lerp = |a: Vec2, b: Vec2| Vec2 {
            x: if corner_norm.x == 1.0 { b.x } else { a.x },
            y: if corner_norm.y == 1.0 { b.y } else { a.y },
        };
        // `pos_min` is the window's upper-left and `pos_max` its lower-right.
        let pos_min = lerp(corner_target, pos);
        let pos_max = lerp(
            Vec2 {
                x: pos.x + size.x,
                y: pos.y + size.y,
            },
            corner_target,
        );
        let min_size = if always_auto {
            1.0
        } else {
            self.style.window_min_size.x
        };
        let min_size_y = if always_auto {
            1.0
        } else {
            self.style.window_min_size.y
        };
        (
            pos_min,
            Vec2 {
                x: (pos_max.x - pos_min.x).max(min_size),
                y: (pos_max.y - pos_min.y).max(min_size_y),
            },
        )
    }

    /// `UpdateWindowManualResize`: the two lower grips and the four borders, when the window can be
    /// resized. Returns the position and size the drag asks for, if any.
    ///
    /// The borders are not highlighted while hovered: the reference waits 0.04 s before showing that
    /// and this port has no cursor to show it with, so only the drag is modelled.
    fn update_resize(
        &mut self,
        name: &str,
        pos: Vec2,
        size: Vec2,
        always_auto: bool,
    ) -> Option<(Vec2, Vec2)> {
        let draw_size = (self.font.size * 1.10)
            .max(self.style.window_rounding + 1.0)
            .trunc();
        let grip_hover_inner = (draw_size * 0.75).trunc();
        let grip_hover_outer = self.style.window_border_hover_padding * 3.0;
        let border_hover = self.style.window_border_hover_padding;
        let mouse = self.io.mouse_pos.unwrap_or_default();
        let base_id = self.get_id("#RESIZE");
        // A handle drag whose button is already up is over: clearing it here, before the handles are
        // tested, is what lets the frame that releases still hover the one under the mouse.
        if let Some(id) = self.active_id {
            if id >= base_id && id < base_id + 8 && !self.io.mouse_down[0] {
                self.active_id = None;
            }
        }
        let mut result = None;

        // The two lower grips.
        let grip_defs = [
            (Vec2 { x: 1.0, y: 1.0 }, Vec2 { x: -1.0, y: -1.0 }),
            (Vec2 { x: 0.0, y: 1.0 }, Vec2 { x: 1.0, y: -1.0 }),
        ];
        let grip_count = if self.io.config_windows_resize_from_edges {
            2
        } else {
            1
        };
        for (index, (corner_n, inner_dir)) in grip_defs.iter().enumerate().take(grip_count) {
            let corner = Vec2 {
                x: pos.x + size.x * corner_n.x,
                y: pos.y + size.y * corner_n.y,
            };
            let mut rect_min = Vec2 {
                x: corner.x - inner_dir.x * grip_hover_outer,
                y: corner.y - inner_dir.y * grip_hover_outer,
            };
            let mut rect_max = Vec2 {
                x: corner.x + inner_dir.x * grip_hover_inner,
                y: corner.y + inner_dir.y * grip_hover_inner,
            };
            if rect_min.x > rect_max.x {
                core::mem::swap(&mut rect_min.x, &mut rect_max.x);
            }
            if rect_min.y > rect_max.y {
                core::mem::swap(&mut rect_min.y, &mut rect_max.y);
            }
            let grip_id = base_id + index as u32;
            let (hovered, held) = self.item_behavior(name, grip_id, rect_min, rect_max);
            if hovered && self.io.mouse_clicked[0] {
                self.active_click_offset = Vec2 {
                    x: mouse.x - rect_min.x,
                    y: mouse.y - rect_min.y,
                };
            }
            if held {
                // `ImLerp(InnerDir * grip_hover_outer_size, InnerDir * -grip_hover_inner_size,
                // CornerPosN)`: outward on the axes the corner is at 0, inward where it is at 1.
                let reach = |dir: f32, corner_at: f32| {
                    if corner_at == 1.0 {
                        dir * -grip_hover_inner
                    } else {
                        dir * grip_hover_outer
                    }
                };
                let corner_target = Vec2 {
                    x: mouse.x - self.active_click_offset.x + reach(inner_dir.x, corner_n.x),
                    y: mouse.y - self.active_click_offset.y + reach(inner_dir.y, corner_n.y),
                };
                result =
                    Some(self.resize_from_corner(pos, size, corner_target, *corner_n, always_auto));
            }
            if hovered || held {
                // The lower-left grip is the north-east/south-west one.
                self.set_mouse_cursor(if index == 1 {
                    crate::io::MouseCursor::ResizeNESW
                } else {
                    crate::io::MouseCursor::ResizeNWSE
                });
            }
            if held || hovered || index == 0 {
                let col = if held {
                    self.style.color_u32(crate::style::Col::ResizeGripActive)
                } else if hovered {
                    self.style.color_u32(crate::style::Col::ResizeGripHovered)
                } else {
                    self.style.color_u32(crate::style::Col::ResizeGrip)
                };
                if col & 0xFF00_0000 != 0 {
                    self.resize_grip_cols[index] = col;
                }
            }
        }

        // The four borders, laid out as `GetResizeBorderRect` does.
        if !self.io.config_windows_resize_from_edges {
            self.active_id_expire();
            return result;
        }
        let far = Vec2 {
            x: pos.x + size.x,
            y: pos.y + size.y,
        };
        let prev_hovered_border = self.resize_border_prev;
        self.resize_border_prev = -1;
        let border_defs = [
            (0usize, 0usize, Vec2 { x: 0.0, y: 0.0 }),
            (1, 0, Vec2 { x: 1.0, y: 0.0 }),
            (2, 1, Vec2 { x: 0.0, y: 0.0 }),
            (3, 1, Vec2 { x: 0.0, y: 1.0 }),
        ];
        for (border_n, axis, corner_norm) in border_defs {
            let border_rect = match border_n {
                0 => (
                    Vec2 {
                        x: pos.x - border_hover,
                        y: pos.y + grip_hover_inner,
                    },
                    Vec2 {
                        x: pos.x + border_hover,
                        y: far.y - grip_hover_inner,
                    },
                ),
                1 => (
                    Vec2 {
                        x: far.x - border_hover,
                        y: pos.y + grip_hover_inner,
                    },
                    Vec2 {
                        x: far.x + border_hover,
                        y: far.y - grip_hover_inner,
                    },
                ),
                2 => (
                    Vec2 {
                        x: pos.x + grip_hover_inner,
                        y: pos.y - border_hover,
                    },
                    Vec2 {
                        x: far.x - grip_hover_inner,
                        y: pos.y + border_hover,
                    },
                ),
                _ => (
                    Vec2 {
                        x: pos.x + grip_hover_inner,
                        y: far.y - border_hover,
                    },
                    Vec2 {
                        x: far.x - grip_hover_inner,
                        y: far.y + border_hover,
                    },
                ),
            };
            let border_id = base_id + border_n as u32 + 4;
            let (hovered_raw, held) =
                self.item_behavior(name, border_id, border_rect.0, border_rect.1);
            // `g.HoveredIdTimer`: zero on the frame the hover starts, then it accumulates.
            self.resize_hover_timer = if hovered_raw && prev_hovered_border == border_n as i32 {
                self.resize_hover_timer + self.io.delta_time
            } else {
                0.0
            };
            if hovered_raw {
                self.resize_border_prev = border_n as i32;
            }
            let hovered = hovered_raw && self.resize_hover_timer > 0.04;
            if hovered || held {
                self.set_mouse_cursor(if axis == 0 {
                    crate::io::MouseCursor::ResizeEW
                } else {
                    crate::io::MouseCursor::ResizeNS
                });
            }
            if hovered && self.io.mouse_clicked[0] {
                self.active_click_offset = Vec2 {
                    x: mouse.x - border_rect.0.x,
                    y: mouse.y - border_rect.0.y,
                };
            }
            if held {
                // `border_target = window->Pos` with the dragged axis replaced by the mouse.
                let mut border_target = pos;
                if axis == 0 {
                    border_target.x = mouse.x - self.active_click_offset.x + border_hover;
                } else {
                    border_target.y = mouse.y - self.active_click_offset.y + border_hover;
                }
                result = Some(self.resize_from_corner(
                    pos,
                    size,
                    border_target,
                    corner_norm,
                    always_auto,
                ));
            }
        }
        self.active_id_expire();
        result
    }

    /// `ButtonBehavior` over a rectangle: hover, press and release.
    fn item_behavior(
        &mut self,
        name: &str,
        id: ImGuiID,
        rect_min: Vec2,
        rect_max: Vec2,
    ) -> (bool, bool) {
        let mouse = self.io.mouse_pos.unwrap_or_default();
        // The reference's ItemAdd/ButtonBehavior test the handle's rectangle, which reaches outside
        // the window on purpose: a border can be grabbed from the pixels just beyond it. Requiring
        // the window to be hovered would make those pixels dead.
        let _ = name;
        let inside = mouse.x >= rect_min.x
            && mouse.x < rect_max.x
            && mouse.y >= rect_min.y
            && mouse.y < rect_max.y;
        let hovered = inside && (self.active_id.is_none() || self.active_id == Some(id));
        if hovered {
            self.hovered_id = Some(id);
        }
        if hovered && self.io.mouse_clicked[0] {
            self.active_id = Some(id);
        }
        let held = self.active_id == Some(id) && self.io.mouse_down[0];
        (hovered, held)
    }

    /// Clears the active id once the button is up.
    fn active_id_expire(&mut self) {
        if self.active_id.is_some() && !self.io.mouse_down[0] {
            self.active_id = None;
        }
    }

    /// `ImGui::RenderResizeGrips`: the two lower corner grips, painted with the colours the input
    /// pass decided. Resizing from the lower-left grip is not wired up yet.
    fn draw_resize_grips(&mut self, pos: Vec2, size: Vec2) {
        let draw_size = (self.font.size * 1.10)
            .max(self.style.window_rounding + 1.0)
            .trunc();
        let border_inner = (self.style.window_border_size * 0.5).round();
        let rounding = self.style.window_rounding;
        let grip_defs = [
            (Vec2 { x: 1.0, y: 1.0 }, Vec2 { x: -1.0, y: -1.0 }),
            (Vec2 { x: 0.0, y: 1.0 }, Vec2 { x: 1.0, y: -1.0 }),
        ];
        for (index, (corner_n, inner_dir)) in grip_defs.iter().enumerate() {
            let col = self.resize_grip_cols[index];
            if col == 0 {
                continue;
            }
            let corner = Vec2 {
                x: pos.x + size.x * corner_n.x,
                y: pos.y + size.y * corner_n.y,
            };
            // Two points along the corner's edges, then the arc of the window rounding, which at
            // rounding 0 collapses onto the corner itself.
            let along = |a: f32, b: f32| Vec2 {
                x: corner.x + inner_dir.x * a,
                y: corner.y + inner_dir.y * b,
            };
            let mut points = Vec::with_capacity(3);
            if index & 1 == 1 {
                points.push(along(border_inner, draw_size));
                points.push(along(draw_size, border_inner));
            } else {
                points.push(along(draw_size, border_inner));
                points.push(along(border_inner, draw_size));
            }
            // PathArcToFast with a zero radius pushes the centre once, so the shape is a
            // triangle rather than a fan.
            points.push(Vec2 {
                x: corner.x + inner_dir.x * (rounding + border_inner),
                y: corner.y + inner_dir.y * (rounding + border_inner),
            });
            self.draw_list.add_convex_poly_filled_aa(&points, col);
        }
    }

    fn clamp_window_pos(&self, pos: Vec2, size: Vec2) -> Vec2 {
        let pad = Vec2 {
            x: self
                .style
                .display_window_padding
                .x
                .max(self.style.display_safe_area_padding.x),
            y: self
                .style
                .display_window_padding
                .y
                .max(self.style.display_safe_area_padding.y),
        };
        let min = Vec2 {
            x: pad.x - size.x,
            y: pad.y - size.y,
        };
        let max = Vec2 {
            x: self.io.display_size.x - pad.x,
            y: self.io.display_size.y - pad.y,
        };
        Vec2 {
            x: pos.x.clamp(min.x, max.x),
            y: pos.y.clamp(min.y, max.y),
        }
    }

    /// `GetColorU32(title_bar_is_highlight ? ImGuiCol_TitleBgActive : ImGuiCol_TitleBg)`: the
    /// focused window's bar stands out. Only the focused-window part of `title_bar_is_highlight`
    /// is modelled.
    fn title_bar_color(&self, name: &str) -> u32 {
        if self.focused_window.as_deref() == Some(name) {
            self.style.color_u32(crate::style::Col::TitleBgActive)
        } else {
            self.style.color_u32(crate::style::Col::TitleBg)
        }
    }

    pub fn window(&self) -> &Window {
        self.windows.last().expect("no window is open")
    }

    /// `GetContentRegionAvail` inside the current window.
    pub fn content_avail(&self) -> Vec2 {
        self.window().content_size
    }

    /// `GetCursorScreenPos`.
    pub fn cursor_screen_pos(&self) -> Vec2 {
        self.window().cursor
    }

    pub fn item_rect_min(&self) -> Vec2 {
        self.window().last_min
    }

    pub fn item_rect_max(&self) -> Vec2 {
        self.window().last_max
    }

    /// `ImGui::Dummy`: an item of a given size that advances the cursor.
    pub fn dummy(&mut self, size: Vec2) {
        let spacing = self.style.item_spacing;
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        // `SkipItems`: a collapsed window does not lay anything out.
        if window.collapsed {
            return;
        }
        let min = window.cursor;
        let max = Vec2 {
            x: min.x + size.x,
            y: min.y + size.y,
        };
        // An item placed at the line start begins a new line: the line height starts over.
        if min.x == window.line_start_x {
            window.line_height = 0.0;
        }
        window.last_min = min;
        window.last_max = max;
        window.has_item = true;
        window.line_height = window.line_height.max(size.y);
        // `ItemSize`: remember the furthest point the layout reached; that is the content extent.
        window.cursor_max = Vec2 {
            x: window.cursor_max.x.max(max.x),
            y: window.cursor_max.y.max(max.y),
        };
        // The line ends here: the cursor moves below it and back to the line start.
        window.cursor = Vec2 {
            x: window.line_start_x,
            y: min.y + window.line_height + spacing.y,
        };
    }

    /// `ImGui::SameLine`: the next item goes to the right of the previous one.
    pub fn same_line(&mut self) {
        let spacing = self.style.item_spacing;
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        if window.has_item {
            window.cursor = Vec2 {
                x: window.last_max.x + spacing.x,
                y: window.last_min.y,
            };
        }
    }

    /// `SetCursorScreenPos`: place the next item at an absolute position.
    pub fn set_cursor_screen_pos(&mut self, pos: Vec2) {
        if let Some(window) = self.windows.last_mut() {
            window.cursor = pos;
        }
    }

    pub fn last_item_hovered(&self) -> bool {
        self.last_item_hovered
    }

    pub fn last_item_active(&self) -> bool {
        self.last_item_active
    }

    /// `GetFrameHeight`: the height of a framed widget.
    fn frame_height(&self) -> f32 {
        self.font.size + self.style.frame_padding.y * 2.0
    }

    /// The hover/active/press logic shared by framed widgets (`ButtonBehavior`).
    fn interact(&mut self, id: ImGuiID, size: Vec2) -> bool {
        let (min, window_pos, window_size) = {
            let window = self.window();
            (window.cursor, window.pos, window.size)
        };
        let max = Vec2 {
            x: min.x + size.x,
            y: min.y + size.y,
        };

        let over_window = self
            .io
            .mouse_pos
            .map(|m| {
                m.x >= window_pos.x
                    && m.x < window_pos.x + window_size.x
                    && m.y >= window_pos.y
                    && m.y < window_pos.y + window_size.y
            })
            .unwrap_or(false);
        let hovered = over_window
            && self
                .io
                .mouse_pos
                .map(|m| m.x >= min.x && m.x < max.x && m.y >= min.y && m.y < max.y)
                .unwrap_or(false);

        if hovered {
            self.hovered_id = Some(id);
        }
        if hovered && self.io.mouse_clicked[0] {
            self.active_id = Some(id);
        }

        // On release the reference clears the active id before returning, so `IsItemActive()`
        // queried afterwards is already false on that frame.
        let was_active = self.active_id == Some(id);
        let mut pressed = false;
        if was_active && self.io.mouse_released[0] {
            pressed = hovered;
            self.active_id = None;
        }
        let active = self.active_id == Some(id);

        self.dummy(size);
        self.last_item_hovered = hovered;
        self.last_item_active = active;
        pressed
    }

    /// `ImGui::Button`: a framed label. Returns true on the frame the mouse is released inside.
    pub fn button(&mut self, label: &str) -> bool {
        let id = self.get_id(label);
        let padding = self.style.frame_padding;
        let text = self.font.calc_text_size(label, self.font.size);
        let size = Vec2 {
            x: text.x + padding.x * 2.0,
            y: self.font.size + padding.y * 2.0,
        };
        let min = self.window().cursor;
        let pressed = self.interact(id, size);

        // Draw it: a filled frame in the state colour, then the label. FrameBorderSize is 0 in the
        // default style, so there is no border to draw.
        let max = Vec2 {
            x: min.x + size.x,
            y: min.y + size.y,
        };
        let frame_color = if self.last_item_active {
            self.style.color_u32(crate::style::Col::ButtonActive)
        } else if self.last_item_hovered {
            self.style.color_u32(crate::style::Col::ButtonHovered)
        } else {
            self.style.color_u32(crate::style::Col::Button)
        };
        self.draw_list.prim_rect(min, max, frame_color);
        // A frame border, when the style asks for one, is the same aligned two-ring frame a window
        // border uses.
        if self.style.frame_border_size > 0.0 {
            self.draw_list.add_aligned_rect_border(
                min,
                max,
                self.style.color_u32(crate::style::Col::Border),
                self.style.frame_border_size,
            );
        }
        let font = self.font.clone();
        let text_color = self.style.color_u32(crate::style::Col::Text);
        self.draw_list.add_text(
            &font,
            font.size,
            Vec2 {
                x: min.x + padding.x,
                y: min.y + padding.y,
            },
            text_color,
            label,
        );
        pressed
    }

    /// `ImGui::SliderFloat` (horizontal, not logarithmic): the value follows the mouse inside the
    /// usable range, with the reference's grab region and its 3-decimal rounding of the format.
    pub fn slider_float(
        &mut self,
        label: &str,
        value: &mut f32,
        min_value: f32,
        max_value: f32,
        width: f32,
    ) -> bool {
        const GRAB_PADDING: f32 = 2.0;
        const DECIMALS: f32 = 1000.0;

        let id = self.get_id(label);
        let frame_height = self.frame_height();
        let text = self.font.calc_text_size(label, self.font.size);
        let item_size = Vec2 {
            x: width + self.style.item_inner_spacing.x + text.x,
            y: frame_height,
        };

        let (min_pos, window_pos, window_size) = {
            let window = self.window();
            (window.cursor, window.pos, window.size)
        };
        let frame_max = Vec2 {
            x: min_pos.x + width,
            y: min_pos.y + frame_height,
        };

        let over_window = self
            .io
            .mouse_pos
            .map(|m| {
                m.x >= window_pos.x
                    && m.x < window_pos.x + window_size.x
                    && m.y >= window_pos.y
                    && m.y < window_pos.y + window_size.y
            })
            .unwrap_or(false);
        let hovered = over_window
            && self
                .io
                .mouse_pos
                .map(|m| {
                    m.x >= min_pos.x && m.x < frame_max.x && m.y >= min_pos.y && m.y < frame_max.y
                })
                .unwrap_or(false);

        if hovered {
            self.hovered_id = Some(id);
        }
        if hovered && self.io.mouse_clicked[0] {
            self.active_id = Some(id);
            self.slider_just_activated = true;
        }

        let slider_sz = (frame_max.x - min_pos.x) - GRAB_PADDING * 2.0;
        let grab_sz = self.style.grab_min_size.min(slider_sz);
        let usable_sz = slider_sz - grab_sz;
        let usable_pos_min = min_pos.x + GRAB_PADDING + grab_sz * 0.5;
        let usable_pos_max = frame_max.x - GRAB_PADDING - grab_sz * 0.5;

        let was_active = self.active_id == Some(id);
        let mut changed = false;
        if was_active {
            if let Some(mouse) = self.io.mouse_pos {
                if self.slider_just_activated {
                    let ratio = if max_value > min_value {
                        (*value - min_value) / (max_value - min_value)
                    } else {
                        0.0
                    };
                    let grab_pos = usable_pos_min + (usable_pos_max - usable_pos_min) * ratio;
                    let around = mouse.x >= grab_pos - grab_sz * 0.5 - 1.0
                        && mouse.x <= grab_pos + grab_sz * 0.5 + 1.0;
                    self.slider_grab_offset = if around { mouse.x - grab_pos } else { 0.0 };
                    self.slider_just_activated = false;
                }
                if usable_sz > 0.0 {
                    let t = ((mouse.x - self.slider_grab_offset - usable_pos_min) / usable_sz)
                        .clamp(0.0, 1.0);
                    let raw = min_value + t * (max_value - min_value);
                    // `RoundScalarWithFormatT` with the "%.3f" default format.
                    let rounded = (raw * DECIMALS).round() / DECIMALS;
                    if *value != rounded {
                        *value = rounded;
                        changed = true;
                    }
                }
            }
        }

        if self.io.mouse_released[0] && was_active {
            self.active_id = None;
        }
        let active = self.active_id == Some(id);

        self.dummy(item_size);
        self.last_item_hovered = hovered;
        self.last_item_active = active;
        changed
    }

    /// `ImGui::Text` for an unformatted string: an item as wide as the text, no interaction.
    pub fn label(&mut self, text: &str) {
        let size = self.font.calc_text_size(text, self.font.size);
        self.dummy(size);
    }

    /// `ImGui::Checkbox`: a square box plus a label. Returns true on the frame it is toggled.
    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> bool {
        let id = self.get_id(label);
        let frame = self.frame_height();
        let text = self.font.calc_text_size(label, self.font.size);
        let size = Vec2 {
            x: frame + self.style.item_inner_spacing.x + text.x,
            y: frame,
        };
        let pressed = self.interact(id, size);
        if pressed {
            *value = !*value;
        }
        pressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_nest_like_the_reference() {
        let mut ctx = Context::new();
        assert_eq!(ctx.current_id(), 0);
        assert_eq!(ctx.get_id("Button"), hash_str_utf8("Button", 0));

        ctx.push_id("Window");
        let window_id = hash_str_utf8("Window", 0);
        assert_eq!(ctx.current_id(), window_id);
        assert_eq!(ctx.get_id("Button"), hash_str_utf8("Button", window_id));

        ctx.push_id("Child");
        let child_id = hash_str_utf8("Child", window_id);
        assert_eq!(ctx.current_id(), child_id);
        assert_eq!(ctx.get_id("Button"), hash_str_utf8("Button", child_id));

        ctx.pop_id();
        ctx.pop_id();
        assert_eq!(ctx.id_stack_depth(), 0);
        assert_eq!(ctx.current_id(), 0);
    }

    #[test]
    fn frame_counter_advances() {
        let mut ctx = Context::new();
        ctx.begin_frame();
        ctx.end_frame();
        ctx.begin_frame();
        assert_eq!(ctx.frame_count(), 2);
    }

    /// The numbers are the reference's, printed by `rust/tools/conformance` for `window_layout`.
    #[test]
    fn window_and_cursor_geometry_matches_reference() {
        let mut ctx = Context::new();
        ctx.begin_frame();

        ctx.begin_window(
            "W",
            Some(Vec2 { x: 60.0, y: 60.0 }),
            Some(Vec2 { x: 300.0, y: 200.0 }),
        );
        assert_eq!(ctx.window().content_origin, Vec2 { x: 68.0, y: 68.0 });
        assert_eq!(ctx.content_avail(), Vec2 { x: 284.0, y: 184.0 });
        assert_eq!(ctx.cursor_screen_pos(), Vec2 { x: 68.0, y: 68.0 });

        ctx.dummy(Vec2 { x: 100.0, y: 20.0 });
        assert_eq!(ctx.item_rect_min(), Vec2 { x: 68.0, y: 68.0 });
        assert_eq!(ctx.item_rect_max(), Vec2 { x: 168.0, y: 88.0 });
        assert_eq!(ctx.cursor_screen_pos(), Vec2 { x: 68.0, y: 92.0 });

        ctx.same_line();
        assert_eq!(ctx.cursor_screen_pos(), Vec2 { x: 176.0, y: 68.0 });

        ctx.dummy(Vec2 { x: 50.0, y: 10.0 });
        assert_eq!(ctx.item_rect_max(), Vec2 { x: 226.0, y: 78.0 });
        assert_eq!(ctx.cursor_screen_pos(), Vec2 { x: 68.0, y: 92.0 });
        ctx.end_window();

        // Second frame without an explicit size: the window keeps what it had.
        ctx.begin_window("W", None, None);
        assert_eq!(ctx.window().pos, Vec2 { x: 60.0, y: 60.0 });
        assert_eq!(ctx.window().size, Vec2 { x: 300.0, y: 200.0 });
        ctx.end_window();
    }
}
