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
/// `g_ImGuiDirCosSin[ImGuiDir_Up]`: the sort arrow of a column sorted ascending.
const ARROW_UP_COS: f32 = 0.0;
const ARROW_UP_SIN: f32 = -1.0;

/// `Shortcut(ImGuiMod_Ctrl | key)` reduced to the ones a text field acts on.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Shortcut {
    None,
    SelectAll,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
}

/// `FindRenderedTextEnd`: a label stops at `##`, so the part after it identifies the widget without
/// being drawn.
fn rendered_text(label: &str) -> &str {
    match label.find("##") {
        Some(at) => &label[..at],
        None => label,
    }
}

/// `FindBestWindowPosForPopupEx` with the combo box policy: below and toward the right first, then
/// above it, then the two left-aligned variants, taking the first that fits the work area.
fn combo_popup_pos(outer: (Vec2, Vec2), avoid: (Vec2, Vec2), size: Vec2) -> Vec2 {
    let fits = |pos: Vec2| {
        pos.x >= outer.0.x
            && pos.y >= outer.0.y
            && pos.x + size.x <= outer.1.x
            && pos.y + size.y <= outer.1.y
    };
    let down = Vec2 {
        x: avoid.0.x,
        y: avoid.1.y,
    };
    let up_same = Vec2 {
        x: avoid.0.x,
        y: avoid.0.y - size.y,
    };
    let down_left = Vec2 {
        x: avoid.1.x - size.x,
        y: avoid.1.y,
    };
    let up_left = Vec2 {
        x: avoid.1.x - size.x,
        y: avoid.0.y - size.y,
    };
    for candidate in [down, up_same, down_left, up_left] {
        if fits(candidate) {
            return candidate;
        }
    }
    Vec2 {
        x: down.x.clamp(outer.0.x, (outer.1.x - size.x).max(outer.0.x)),
        y: down.y.clamp(outer.0.y, (outer.1.y - size.y).max(outer.0.y)),
    }
}
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
    /// `ImGuiWindowFlags_MenuBar`: a bar along the top of the window, which takes its height out of
    /// the content area.
    pub const MENU_BAR: Self = Self(1 << 7);
    /// `ImGuiWindowFlags_Tooltip`: a tooltip window, which takes no mouse and no keyboard focus.
    pub const TOOLTIP: Self = Self(1 << 6);
    /// `ImGuiWindowFlags_Popup`: draws with the popup background.
    pub const POPUP: Self = Self(1 << 5);

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
    /// `window->DC.MenuColumns`: the column widths the menu item rows are laid out with.
    menu_columns: crate::menu_columns::MenuColumns,
    /// `window->LastFrameActive`: the frame the window was last submitted on, which tells `Begin`
    /// whether the window is reappearing and has to forget its measurements.
    last_frame: u64,
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
    /// `window->MenuBarHeight`: zero without the flag, the frame height with it.
    pub menu_bar_height: f32,
    /// `window->DC.LayoutType == Horizontal`: a menu bar lays its items out in a row.
    pub layout_horizontal: bool,
    /// `ImGuiWindowFlags_ChildWindow`: a region inside another window. A child has its own draw
    /// list and keeps no state between frames.
    pub is_child: bool,
    /// `pos + style.WindowPadding`, minus the scroll.
    pub content_origin: Vec2,
    /// `size - style.WindowPadding * 2`, what `GetContentRegionAvail` returns inside the window.
    pub content_size: Vec2,
    /// `window->DC.MenuColumns`: the column widths the rows of a menu are laid out with. It lives on
    /// the window because it survives between frames.
    pub menu_columns: crate::menu_columns::MenuColumns,
    cursor: Vec2,
    cursor_start: Vec2,
    cursor_max: Vec2,
    line_start_x: f32,
    line_height: f32,
    last_min: Vec2,
    last_max: Vec2,
    has_item: bool,
    /// `window->ClipRect` as far as the items of a table cell need it: the reference narrows it to
    /// the column while cells are submitted, so text outside the column is dropped in software.
    clip_rect: Option<[f32; 4]>,
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
    /// `g.CurrentTable`: the table being built, with the columns its layout already resolved.
    table: Option<crate::tables::Table>,
    /// What `BeginTable` changed on the host window and has to put back: the line start, so the
    /// item after the table goes back to the window's content column.
    table_host: Option<TableHost>,
    /// `ImDrawListSplitter` for a table: one index channel per column. The reference draws each
    /// cell into its column's channel and merges them in column order at `EndTable`, which is why
    /// the index buffer of a table is *not* in drawing order while its vertices are.
    table_indices: Vec<Vec<crate::draw_list::ImDrawIdx>>,
    /// The background channel of the splitter (`TABLE_DRAW_CHANNEL_BG0`), which the row backgrounds
    /// use. It merges before every column channel.
    table_bg_indices: Vec<crate::draw_list::ImDrawIdx>,
    /// Where the current cell's indices start in the list, so they can be moved to their channel.
    cell_index_start: usize,
    /// The identifier of the table being built, which keys the per-column content extents that have
    /// to survive to the next frame (the reference keeps them in the `ImGuiTable` it reuses).
    table_id: Option<String>,
    /// `column->ContentMaxX*` from the last frame a table was built, by table identifier.
    table_content_max_x: std::collections::HashMap<String, Vec<f32>>,
    /// `g.HoveredIdPreviousFrame` for the header cells: headers use `AllowOverlap`, so a header only
    /// reports hover when it was the hovered item on the frame before.
    header_hovered_last: Option<(String, usize)>,
    /// The header the mouse is over on this frame, which becomes the previous frame's id at once.
    header_hovered_next: Option<(String, usize)>,
    /// `table_instance->LastOuterHeight`: the height the table had on the previous frame, which the
    /// hover test of the next one uses because the current height is not known yet.
    table_last_outer_height: std::collections::HashMap<String, f32>,
    /// `table_instance->HoveredRowNext` from the last frame, which is what the next layout reads.
    table_hovered_row_next: std::collections::HashMap<String, i32>,
    /// The size of the inner child window a scrolling table opened, `None` for a plain table: it is
    /// what `EndTable` must close and the size the host advances by.
    table_child_size: Option<Vec2>,
    /// `Bg0ClipRectForDrawCmd`: the host window's clip, which the row backgrounds carry and which is
    /// only different from the table's own clip when the table scrolls inside an inner window.
    table_outer_clip: Option<[f32; 4]>,
    /// `column->SortOrder`, `column->SortDirection` and the two dirty flags of the sort specs from the
    /// last frame a table was built, by table identifier: the reference keeps them in the `ImGuiTable`
    /// it reuses (and reloads them from its settings), so a sort applied by a click survives to the
    /// frames that draw it and only a first frame (or a click) asks the caller to sort.
    table_sort_state: std::collections::HashMap<String, crate::tables::TableSortState>,
    /// `column->DisplayOrder` plus the reorder request a drag queued (`ReorderColumn` and
    /// `ReorderColumnDstOrder`), by table identifier: the reference keeps them in the `ImGuiTable` it
    /// reuses, and the move is applied by the next `BeginTable`.
    table_column_order: std::collections::HashMap<String, (Vec<i32>, i32, i32)>,
    /// `g.ActiveId` for the header cells: the one held with the mouse, until the button is released.
    header_active: Option<(String, usize)>,
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
    /// `SetNextWindowFocus` for the window about to start, which `Begin` applies as `FocusWindow`.
    next_focus: bool,
    /// How many `BeginDisabled` are open, and the `Style::Alpha` the outermost one found.
    disabled_depth: u32,
    disabled_alpha_backup: Option<f32>,
    /// `SetNextWindowBgAlpha`: the alpha the next window background is painted with.
    next_bg_alpha: Option<f32>,
    /// `SetNextWindowCollapsed`: the state `Begin` applies to the window it starts.
    next_collapsed: Option<bool>,
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
    /// `window->DC.StateStorage`: the open or closed state of every tree node and collapsing header.
    item_states: Vec<(ImGuiID, bool)>,
    /// The host each closed list belongs to, so a child's list follows its host's.
    list_parents: Vec<Option<String>>,
    /// `g.OpenPopupStack`: the popup that is open, if any.
    popup_open: Option<ImGuiID>,
    /// Set when a click landed outside the popup; the popup closes on the next frame.
    popup_pending_close: bool,
    /// Whether the popup is being laid out for the first time, so nothing is drawn.
    popup_first_frame: bool,
    /// The lists set aside while a nested window owns the current one.
    list_stack: Vec<crate::draw_list::DrawList>,
    /// What a child region remembers between frames: its scroll and how big its content was.
    child_memory: Vec<(String, Vec2, Vec2, (Vec2, Vec2))>,
    /// The host window's list, set aside while a child owns the current one.
    parent_list: Option<crate::draw_list::DrawList>,
    /// While a child is open, the name of the window hosting it.
    pending_child_host: Option<String>,
    /// The popup's rectangle from the last frame, which sits above every window for the mouse.
    popup_rect: Option<(String, (Vec2, Vec2))>,
    /// `SetNextWindowSizeConstraints`: the smallest size the next window may take.
    next_size_min: Option<Vec2>,
    /// The horizontal window padding to restore once the combo popup closes.
    combo_saved_padding: Option<f32>,
    /// The window a popup belongs to, which takes the focus back when the popup closes.
    popup_host: Option<String>,
    /// The window that hosts the child menu currently open. `IsRootOfOpenMenuSet` answers with it:
    /// while a menu of this window is open, its siblings open on hover instead of waiting for a click.
    child_menu_host: Option<String>,
    /// `g.BeginMenuDepth`: menus are recycled by depth, and it names their windows.
    menu_depth: usize,
    /// `parent_window->ClipRect.Max.y` of the menu bar while its entries are submitted: it is the
    /// reference position a child menu is placed below.
    menu_bar_clip_max_y: Option<f32>,
    /// The popup name egin_popup opened, carried to nd_popup for the rectangle.
    last_popup_name: Option<String>,
    /// `g.InputTextState`: the edit state of the field that owns the keyboard, if any.
    input_text_state: Option<crate::input_text::InputTextState>,
    /// `SetNextItemWidth`: the width the next item asks for, consumed by the item that reads it.
    next_item_width: Option<f32>,
    /// `ImGui::PushItemWidth` stack: the widths later items take, zero meaning the default.
    /// ImGui::PushStyleColor stack: the colours to restore on each pop.
    color_stack: Vec<(crate::style::Col, u32)>,
    item_width_stack: Vec<f32>,
    /// `g.HoveredIdTimer`: how long the border under the mouse has been hovered. The reference
    /// waits WINDOWS_RESIZE_FROM_EDGES_FEEDBACK_TIMER before it reports a border, so brushing past
    /// an edge on the way to a widget does not light it up.
    resize_hover_timer: f32,
    /// The border hovered last frame, so the timer knows when the hover started.
    resize_border_prev: i32,
    /// window->ResizeBorderHovered and ResizeBorderHeld: which border is grabbed, -1 when none.
    resize_border_hovered: i32,
    resize_border_held: i32,
    /// `g.MovingWindow`: the window that follows the mouse while the move drag lasts.
    moving_window: Option<String>,
    /// The window's draw list: what this frame has drawn so far.
    draw_list: crate::draw_list::DrawList,
    slider_just_activated: bool,
    slider_grab_offset: f32,
    last_item_hovered: bool,
    last_item_active: bool,
}

/// What `BeginTable` has to put back when the table ends: the host window's line start, which the
/// cells move to their own content column (`window->DC.ColumnsOffset` in the reference).
#[derive(Clone, Copy, Debug)]
struct TableHost {
    line_start_x: f32,
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
            table: None,
            table_id: None,
            table_content_max_x: std::collections::HashMap::new(),
            header_hovered_last: None,
            header_hovered_next: None,
            table_last_outer_height: std::collections::HashMap::new(),
            table_hovered_row_next: std::collections::HashMap::new(),
            table_child_size: None,
            table_outer_clip: None,
            table_sort_state: std::collections::HashMap::new(),
            table_column_order: std::collections::HashMap::new(),
            header_active: None,
            table_host: None,
            table_indices: Vec::new(),
            table_bg_indices: Vec::new(),
            cell_index_start: 0,
            memory: Vec::new(),
            next_scroll: None,
            next_flags: None,
            next_focus: false,
            disabled_depth: 0,
            disabled_alpha_backup: None,
            next_bg_alpha: None,
            next_collapsed: None,
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
            item_states: Vec::new(),
            list_parents: Vec::new(),
            popup_open: None,
            popup_pending_close: false,
            popup_first_frame: false,
            list_stack: Vec::new(),
            child_memory: Vec::new(),
            parent_list: None,
            pending_child_host: None,
            popup_rect: None,
            next_size_min: None,
            combo_saved_padding: None,
            popup_host: None,
            child_menu_host: None,
            menu_depth: 0,
            menu_bar_clip_max_y: None,
            input_text_state: None,
            next_item_width: None,
            color_stack: Vec::new(),
            item_width_stack: Vec::new(),
            last_popup_name: None,
            resize_grip_cols: [0; 2],
            resize_hover_timer: 0.0,
            resize_border_prev: -1,
            resize_border_hovered: -1,
            resize_border_held: -1,
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
        // A popup is above everything, so the mouse inside one resolves to it before any window.
        if let Some((name, rect)) = &self.popup_rect {
            if rect.0.x <= mouse.x
                && mouse.x < rect.1.x
                && rect.0.y <= mouse.y
                && mouse.y < rect.1.y
            {
                self.hovered_window = Some(name.clone());
                return;
            }
        }
        // A child is a window of its own and sits above its host, so the mouse inside a child
        // resolves to the child, not to the window hosting it.
        for name in self.display_order.iter().rev() {
            // A child is a window of its own and sits above its host, so the mouse inside a child
            // resolves to the child, not to the window hosting it.
            let prefix = format!("{name}/");
            if let Some((child, _, _, _)) = self.child_memory.iter().find(|(child, _, _, rect)| {
                child.starts_with(&prefix)
                    && rect.0.x <= mouse.x
                    && mouse.x < rect.1.x
                    && rect.0.y <= mouse.y
                    && mouse.y < rect.1.y
            }) {
                found = Some(child.clone());
                break;
            }
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
        // A child's list is submitted right after the list of the window hosting it, which is the
        // order the reference's draw data ends up in.
        let mut queue = self.display_order.clone();
        let mut guard = 0;
        while !queue.is_empty() {
            guard += 1;
            if guard > 1024 {
                break;
            }
            let name = queue.remove(0);
            if let Some((_, list)) = self.frame_lists.iter().find(|(known, _)| *known == name) {
                if !list.is_empty() {
                    let mut list = list.clone();
                    // `_PopUnusedDrawCmd`: a presented list never ends with an empty command.
                    list.pop_unused_draw_cmd();
                    out.push((name.clone(), list));
                }
            }
            for (index, host) in self.list_parents.iter().enumerate() {
                if host.as_deref() == Some(name.as_str()) {
                    if let Some((child, _)) = self.frame_lists.get(index) {
                        queue.push(child.clone());
                    }
                }
            }
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

    /// `ImGui::SetMouseCursor`: what the backend should show for the rest of the frame.
    pub fn set_mouse_cursor(&mut self, cursor: crate::io::MouseCursor) {
        self.io.mouse_cursor = cursor;
    }

    /// SetNextWindowBgAlpha: the alpha the next window background is painted with.
    pub fn set_next_window_bg_alpha(&mut self, alpha: f32) {
        self.next_bg_alpha = Some(alpha);
    }

    /// `ImGui::SetNextWindowFocus`: the next window `Begin` raises to the front and focuses.
    pub fn set_next_window_focus(&mut self) {
        self.next_focus = true;
    }

    /// `ImGui::BeginDisabled`: every colour packed from here on gets `Style::Alpha` multiplied by
    /// `Style::DisabledAlpha` (0.6), and the items inside neither hover nor take a press. Nested calls
    /// only count.
    pub fn begin_disabled(&mut self) {
        if self.disabled_depth == 0 {
            self.disabled_alpha_backup = Some(self.style.alpha);
            self.style.alpha *= self.style.disabled_alpha;
        }
        self.disabled_depth += 1;
    }

    /// `ImGui::EndDisabled`: back to the alpha saved by the outermost [`Context::begin_disabled`].
    pub fn end_disabled(&mut self) {
        if self.disabled_depth == 0 {
            return;
        }
        self.disabled_depth -= 1;
        if self.disabled_depth == 0 {
            if let Some(alpha) = self.disabled_alpha_backup.take() {
                self.style.alpha = alpha;
            }
        }
    }

    /// `ImGui::SetNextWindowCollapsed`: the next window `Begin` starts collapsed or expanded.
    pub fn set_next_window_collapsed(&mut self, collapsed: bool) {
        self.next_collapsed = Some(collapsed);
    }

    /// `ImGui::SetWindowFocus(name)`: the named window comes to the front of the display order and
    /// takes the focus at once, which is the state the next `Begin` reads. An unknown name is a
    /// no-op, like a failed `FindWindowByName` in the reference.
    pub fn set_window_focus(&mut self, name: &str) {
        if let Some(index) = self.display_order.iter().position(|known| known == name) {
            let name = self.display_order.remove(index);
            self.display_order.push(name.clone());
            self.focused_window = Some(name);
        }
    }
    /// `ImGui::SetWindowCollapsed(name, collapsed)` without the condition: the window takes the
    /// state at its next `Begin`, which reads the remembered collapsed flag.
    pub fn set_window_collapsed(&mut self, name: &str, collapsed: bool) {
        if let Some(memory) = WindowMemory::find_mut(&mut self.memory, name) {
            memory.collapsed = collapsed;
        }
    }

    /// `ImGui::SetWindowSize(name, size, cond)` without the condition: the window resizes now, which
    /// the port keeps in the memory it reads at the next `Begin`. Only the positive form is modelled:
    /// the reference truncates the size, and a non-positive axis turns on auto-fit.
    pub fn set_window_size(&mut self, name: &str, size: Vec2) {
        if let Some(memory) = WindowMemory::find_mut(&mut self.memory, name) {
            if size.x > 0.0 {
                memory.size_full.x = size.x.trunc();
            }
            if size.y > 0.0 {
                memory.size_full.y = size.y.trunc();
            }
        }
    }

    /// `ImGui::SetWindowPos(name, pos, cond)` without the condition: the window moves now, which the
    /// port keeps in the memory it reads at the next `Begin`. The reference truncates the position,
    /// and moving a window that is not submitted this frame still takes effect on its next frame.
    pub fn set_window_pos(&mut self, name: &str, pos: Vec2) {
        let pos = Vec2 {
            x: pos.x.trunc(),
            y: pos.y.trunc(),
        };
        if let Some(memory) = WindowMemory::find_mut(&mut self.memory, name) {
            memory.pos = pos;
        }
    }

    /// The `ImGuiWindowFlags` of the next window, as `ImGui::Begin` takes them.
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
            // `DownDurationPrev` first: `IsKeyReleased` reads it on the frame the key comes up.
            data.down_duration_prev = data.down_duration;
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
    /// `PushOverrideID`: put an id on the stack as is, without hashing it onto the current one.
    fn push_override_id(&mut self, id: ImGuiID) {
        self.id_stack.push(id);
    }

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
        // `window_just_activated_by_user`: the window was not submitted on the previous frame, so
        // this is its first frame back. A menu uses it to decide whether its columns are locked or
        // thrown away, which is why a menu that has just been opened settles on its second frame.
        let window_just_activated = match remembered {
            Some(memory) => memory.last_frame + 1 < self.frame_count,
            None => true,
        };
        let remembered_menu_columns = remembered
            .map(|memory| memory.menu_columns)
            .unwrap_or_default();
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
        // `SetNextWindowFocus` (`g.NextWindowData.Focus`): `FocusWindow` raises the window to the
        // front of the display order and gives it the focus, like a click does but without the drag.
        if self.next_focus {
            self.next_focus = false;
            if let Some(index) = self.display_order.iter().position(|known| known == name) {
                let name = self.display_order.remove(index);
                self.display_order.push(name);
            }
            self.focused_window = Some(name.to_string());
        }

        // The arrow asks for the toggle; Begin applies it at the start of the next frame.
        if opts.collapsible && want_collapse_toggle {
            collapsed = !collapsed;
        }
        want_collapse_toggle = false;
        // `SetNextWindowCollapsed`: `Begin` applies the requested state to the window, which is the
        // same field the title-bar arrow toggles.
        if let Some(next_collapsed) = self.next_collapsed.take() {
            collapsed = next_collapsed;
        }

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
        // `CalcWindowSizeAfterConstraint`: the minimum a popup is given keeps it at least as wide
        // as the box it belongs to.
        if let Some(minimum) = self.next_size_min.take() {
            size_full.x = size_full.x.max(minimum.x);
            size_full.y = size_full.y.max(minimum.y);
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
            if let Some((new_pos, new_size)) =
                self.update_resize(name, pos, size_full, content, title_height, always_auto)
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
                if flags.contains(WindowFlags::POPUP) {
                    self.style.popup_bg
                } else if let Some(alpha) = self.next_bg_alpha.take() {
                    // ``SetNextWindowBgAlpha``: the same background with its alpha scaled.
                    self.style
                        .color_u32_alpha(crate::style::Col::WindowBg, alpha)
                } else {
                    self.style.color_u32(crate::style::Col::WindowBg)
                },
            );
        }

        // `RenderWindowDecorations`: a menu bar paints its own background over the top of the window,
        // before the border goes around everything, and takes its height out of the content area.
        let menu_bar_height = if flags.contains(WindowFlags::MENU_BAR) {
            self.frame_height()
        } else {
            0.0
        };
        if menu_bar_height > 0.0 {
            self.draw_list.prim_rect(
                Vec2 {
                    x: pos.x,
                    y: pos.y + title_height,
                },
                Vec2 {
                    x: pos.x + size_full.x,
                    y: pos.y + title_height + menu_bar_height,
                },
                self.style.menu_bar_bg,
            );
            content_origin.y += menu_bar_height;
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
            // `RenderWindowOuterSingleBorder` over the grabbed edge, thicker than the frame.
            let highlight = if self.resize_border_held != -1 {
                Some((self.resize_border_held, true))
            } else if self.resize_border_hovered != -1 {
                Some((self.resize_border_hovered, false))
            } else {
                None
            };
            if let Some((border_n, active)) = highlight {
                self.draw_resize_border_highlight(pos, size, border_n, active);
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
        // The content clip starts at the bar's bottom when there is one: the bar itself already sits
        // inside the border.
        let clip_top = self
            .style
            .window_border_size
            .max(title_height + menu_bar_height);
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
            y: size.y - pad.y * 2.0 - menu_bar_height,
        };
        // window->ID = ImHashStr(name) and the id stack starts from it, which is what makes an
        // item's id depend on its window and a child's name carry the host's hash.
        self.id_stack.push(crate::id::hash_str_utf8(name, 0));
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
            menu_bar_height,
            layout_horizontal: false,
            is_child: false,
            content_origin,
            content_size,
            menu_columns: remembered_menu_columns,
            cursor: content_origin,
            cursor_start: content_origin,
            cursor_max: content_origin,
            line_start_x: content_origin.x,
            line_height: 0.0,
            last_min: Vec2::default(),
            last_max: Vec2::default(),
            has_item: false,
            clip_rect: None,
        });
        // `window->DC.MenuColumns.Update(style.ItemSpacing.x, window_just_activated_by_user)`: the
        // offsets are locked from the previous frame before any item of this one is submitted.
        let spacing = self.style.item_spacing.x;
        if let Some(window) = self.windows.last_mut() {
            window.menu_columns.update(spacing, window_just_activated);
        }

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
    /// `ImGui::BeginChild`: a region inside the current window, with its own layout, its own padding
    /// and **its own draw list**, as the reference gives it. That is what makes a child cheap to
    /// submit and what keeps its content out of its host's list.
    ///
    /// Scrolling inside a child is not wired up yet: its content is clipped by culling against its
    /// own rectangle, but the wheel and the scrollbars are still the parent's.
    pub fn begin_child(&mut self, id: &str, size: Vec2, borders: bool) {
        let pad = self.style.window_padding;
        let Some(parent) = self.windows.last() else {
            return;
        };
        let host = parent.name.clone();
        let pos = parent.cursor;
        // BeginChild names the window %s/%s_%08X: the host's name, the child's and its id.
        let child_name = format!("{host}/{id}_{:08X}", self.get_id(id));
        let inner_rect = (
            pos,
            Vec2 {
                x: pos.x + size.x,
                y: pos.y + size.y,
            },
        );
        // `Begin` decides the scrollbar from the content the child had last frame, exactly as a
        // window does, so a region whose content grew shows its bar one frame later.
        let previous = self
            .child_memory
            .iter()
            .find(|(name, _, _, _)| name == &format!("{host}/{id}_{:08X}", self.get_id(id)));
        let content_before = previous
            .map(|(_, _, content, _)| *content)
            .unwrap_or_default();
        let scroll_before = previous
            .map(|(_, scroll, _, _)| *scroll)
            .unwrap_or_default();
        // The bar sits against the region's own edge, so the outer rectangle is the one before the
        // bar took its width out of the interior.
        let child_rect = inner_rect;
        let mut inner_rect = inner_rect;
        let scrollbar_y = content_before.y + pad.y * 2.0 > inner_rect.1.y - inner_rect.0.y;
        if scrollbar_y {
            inner_rect.1.x -= self.style.scrollbar_size;
        }
        let scroll_max = Vec2 {
            x: (content_before.x + pad.x * 2.0 - (inner_rect.1.x - inner_rect.0.x)).max(0.0),
            y: (content_before.y + pad.y * 2.0 - (inner_rect.1.y - inner_rect.0.y)).max(0.0),
        };
        let mut requested = scroll_before;
        let child_now = format!("{host}/{id}_{:08X}", self.get_id(id));
        // `UpdateMouseWheel`: the wheel scrolls what it is locked to, falling back to whatever is
        // under the mouse, with the same five-font-size step a window uses.
        if self.io.mouse_wheel != 0.0 {
            let owner = self
                .wheeling_window
                .clone()
                .or_else(|| self.hovered_window.clone());
            if owner.as_deref() == Some(child_now.as_str()) && scroll_max.y != 0.0 {
                let amount = self.io.mouse_wheel.abs();
                self.wheeling_release_timer =
                    (self.wheeling_release_timer + amount * 0.70).min(0.70);
                if self.wheeling_window.as_deref() != Some(child_now.as_str()) {
                    self.wheeling_window = Some(child_now.clone());
                    self.wheeling_ref_mouse = self.io.mouse_pos.unwrap_or_default();
                }
                let max_step = (inner_rect.1.y - inner_rect.0.y) * 0.67;
                let scroll_step = (5.0 * self.font.size).min(max_step).trunc();
                requested.y -= self.io.mouse_wheel * scroll_step;
            }
        }
        let scroll = Vec2 {
            x: requested.x.clamp(0.0, scroll_max.x),
            y: requested.y.clamp(0.0, scroll_max.y),
        };
        let content_origin = Vec2 {
            x: inner_rect.0.x + pad.x - scroll.x,
            y: inner_rect.0.y + pad.y - scroll.y,
        };
        let content_size = Vec2 {
            x: inner_rect.1.x - inner_rect.0.x - pad.x * 2.0,
            y: inner_rect.1.y - inner_rect.0.y - pad.y * 2.0,
        };

        // `RenderWindowOuterBorders` and the scrollbars run while the host still owns the list, so a
        // child's border and its scrollbar are part of the host's draw list, not of the child's. The
        // bar is still the child's own item, so its id lives under the child.
        let mut scroll = scroll;
        let mut content_origin = content_origin;
        self.push_id(id);
        if scrollbar_y {
            // The hover search names the child Parent/Child_HASH, so that is the name the bar
            // has to match to be hovered.
            if let Some(seeked) = self.draw_scrollbar(
                &child_name,
                child_rect,
                inner_rect,
                content_before,
                scroll,
                false,
            ) {
                scroll = seeked;
                content_origin = Vec2 {
                    x: inner_rect.0.x + pad.x - scroll.x,
                    y: inner_rect.0.y + pad.y - scroll.y,
                };
            }
        }
        self.pop_id();
        if borders && self.style.child_border_size > 0.0 {
            // The border goes around the whole region, not around its interior.
            self.draw_list.add_aligned_rect_border(
                child_rect.0,
                child_rect.1,
                self.style.color_u32(crate::style::Col::Border),
                self.style.child_border_size,
            );
        }
        // The child takes over the current list; the host's list waits until `end_child`.
        let child_list = self.new_window_draw_list();
        let host_list = core::mem::replace(&mut self.draw_list, child_list);
        self.parent_list = Some(host_list);

        self.windows.push(Window {
            name: child_name,
            pos,
            size,
            size_full: size,
            collapsed: false,
            want_collapse_toggle: false,
            flags: WindowFlags::NONE,
            scroll,
            scroll_max,
            content_extent: content_before,
            scrollbar_y,
            scrollbar_x: false,
            inner_rect,
            menu_bar_height: 0.0,
            layout_horizontal: false,
            is_child: true,
            content_origin,
            content_size,
            // A child region keeps no memory between frames in this port, so its menu columns are
            // rebuilt every frame instead of being locked from the previous one. A menu opened from
            // inside a child still gets its own window, whose columns do persist.
            menu_columns: crate::menu_columns::MenuColumns::default(),
            cursor: content_origin,
            cursor_start: content_origin,
            cursor_max: content_origin,
            line_start_x: content_origin.x,
            line_height: 0.0,
            last_min: Vec2::default(),
            last_max: Vec2::default(),
            has_item: false,
            clip_rect: None,
        });
        self.push_id(id);
        // `InnerClipRect`: the child's own list starts clipped to its rectangle, shrunk by half a
        // border and floored, which is what the reference's first command carries.
        let border = self.style.child_border_size;
        let clip = (
            Vec2 {
                x: (0.5 + inner_rect.0.x + border * 0.5).floor(),
                y: (0.5 + inner_rect.0.y + border * 0.5).floor(),
            },
            Vec2 {
                x: (inner_rect.1.x - border * 0.5).floor(),
                y: (inner_rect.1.y - border * 0.5).floor(),
            },
        );
        // A fresh list starts from the full screen, so this clip is absolute, not intersected.
        self.draw_list.push_clip_rect(clip.0, clip.1, false);
        self.pending_child_host = Some(host);
    }

    /// `BeginChildEx` for the inner window of a scrolling table: no padding, no border, its own draw
    /// list, and the content scrolled by the offset the child had on the previous frame.
    fn begin_table_child(&mut self, id: &str, outer_rect: (Vec2, Vec2), host_clip: [f32; 4]) {
        let Some(parent) = self.windows.last() else {
            return;
        };
        let host = parent.name.clone();
        let table_id = self.get_id(id);
        let child_name = format!("{host}/{id}_{table_id:08X}");
        let size = Vec2 {
            x: outer_rect.1.x - outer_rect.0.x,
            y: outer_rect.1.y - outer_rect.0.y,
        };
        let pos = outer_rect.0;
        // `ImGuiChildFlags_None` with the table's zero padding and border: the child's rectangle is
        // exactly the table's, and the content starts at its top-left corner.
        let inner_rect = outer_rect;
        let content_before = self
            .child_memory
            .iter()
            .find(|(name, _, _, _)| *name == child_name)
            .map(|(_, _, content, _)| *content)
            .unwrap_or_default();
        let scroll_before = self
            .child_memory
            .iter()
            .find(|(name, _, _, _)| *name == child_name)
            .map(|(_, scroll, _, _)| *scroll)
            .unwrap_or_default();
        let scroll_max = Vec2 {
            x: (content_before.x - size.x).max(0.0),
            y: (content_before.y - size.y).max(0.0),
        };
        let scroll = Vec2 {
            x: scroll_before.x.clamp(0.0, scroll_max.x),
            y: scroll_before.y.clamp(0.0, scroll_max.y),
        };
        let content_origin = Vec2 {
            x: inner_rect.0.x - scroll.x,
            y: inner_rect.0.y - scroll.y,
        };
        let content_size = size;
        let child_list = self.new_window_draw_list();
        let host_list = core::mem::replace(&mut self.draw_list, child_list);
        self.parent_list = Some(host_list);
        self.windows.push(Window {
            name: child_name,
            pos,
            size,
            size_full: size,
            collapsed: false,
            want_collapse_toggle: false,
            flags: WindowFlags::NONE,
            scroll,
            scroll_max,
            content_extent: content_before,
            scrollbar_y: false,
            scrollbar_x: false,
            inner_rect,
            menu_bar_height: 0.0,
            layout_horizontal: false,
            is_child: true,
            content_origin,
            content_size,
            menu_columns: crate::menu_columns::MenuColumns::default(),
            cursor: content_origin,
            cursor_start: content_origin,
            cursor_max: content_origin,
            line_start_x: content_origin.x,
            line_height: 0.0,
            last_min: Vec2::default(),
            last_max: Vec2::default(),
            has_item: false,
            clip_rect: None,
        });
        // The reference's `BeginChildEx` pushes the child's window id, which for a table is the
        // instance id the caller handed in: the table's own id.
        self.push_override_id(table_id);
        // `window->InnerClipRect`: the child's rectangle clipped to the parent's clip, floored, which
        // is the first clip its list carries.
        let clip = (
            Vec2 {
                x: (0.5 + inner_rect.0.x).floor().max(host_clip[0]),
                y: (0.5 + inner_rect.0.y).floor().max(host_clip[1]),
            },
            Vec2 {
                x: inner_rect.1.x.floor().min(host_clip[2]),
                y: inner_rect.1.y.floor().min(host_clip[3]),
            },
        );
        self.draw_list.push_clip_rect(clip.0, clip.1, false);
        self.pending_child_host = Some(host);
        self.table_child_size = Some(size);
    }

    /// `EndChild` for the inner window of a scrolling table: the child's extents are what its next
    /// frame's scroll range uses, and the host advances past the table as one item.
    fn end_table_child(&mut self) {
        let Some(size) = self.table_child_size.take() else {
            return;
        };
        self.draw_list.pop_clip_rect();
        if let Some(child) = self.windows.pop() {
            let content = Vec2 {
                x: (child.cursor_max.x - child.cursor_start.x).trunc(),
                y: (child.cursor_max.y - child.cursor_start.y).trunc(),
            };
            let rect = (
                child.pos,
                Vec2 {
                    x: child.pos.x + child.size.x,
                    y: child.pos.y + child.size.y,
                },
            );
            match self
                .child_memory
                .iter_mut()
                .find(|(name, _, _, _)| *name == child.name)
            {
                Some(slot) => {
                    slot.1 = child.scroll;
                    slot.2 = content;
                    slot.3 = rect;
                }
                None => self
                    .child_memory
                    .push((child.name.clone(), child.scroll, content, rect)),
            }
            let child_list = core::mem::take(&mut self.draw_list);
            self.frame_lists.push((child.name.clone(), child_list));
            self.list_parents.push(self.pending_child_host.take());
            self.draw_list = self
                .parent_list
                .take()
                .unwrap_or_else(|| self.new_window_draw_list());
        }
        self.pop_id();
        self.dummy(size);
    }

    /// `ImGui::EndChild`: close the region's list, hand the host's list back, and advance the host
    /// past the region as an item of its size.
    pub fn end_child(&mut self) {
        let Some(child) = self.windows.pop() else {
            return;
        };
        if !child.is_child {
            self.windows.push(child);
            return;
        }
        self.pop_id();
        self.draw_list.pop_clip_rect();
        // CalcWindowContentSizes: the extent the layout reached, measured from the scrolled start.
        let content = Vec2 {
            x: (child.cursor_max.x - child.cursor_start.x).trunc(),
            y: (child.cursor_max.y - child.cursor_start.y).trunc(),
        };
        let rect = (
            child.pos,
            Vec2 {
                x: child.pos.x + child.size.x,
                y: child.pos.y + child.size.y,
            },
        );
        match self
            .child_memory
            .iter_mut()
            .find(|(name, _, _, _)| *name == child.name)
        {
            Some(slot) => {
                slot.1 = child.scroll;
                slot.2 = content;
                slot.3 = rect;
            }
            None => self
                .child_memory
                .push((child.name.clone(), child.scroll, content, rect)),
        }
        let child_list = core::mem::take(&mut self.draw_list);
        self.frame_lists.push((child.name.clone(), child_list));
        self.list_parents.push(self.pending_child_host.take());
        self.draw_list = self
            .parent_list
            .take()
            .unwrap_or_else(|| self.new_window_draw_list());
        self.dummy(child.size);
    }

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
                menu_columns: window.menu_columns,
                last_frame: self.frame_count,
            };
            match WindowMemory::find_mut(&mut self.memory, &window.name) {
                Some(slot) => *slot = entry,
                None => self.memory.push(entry),
            }
            // The frame's list for this window, kept for `render_lists`.
            let list = core::mem::take(&mut self.draw_list);
            self.frame_lists.push((window.name.clone(), list.clone()));
            self.list_parents.push(None);
            self.last_list = list;
            self.id_stack.pop();
        }
    }

    /// `ImGui::EndFrame`: what the reference does once every window has been submitted.
    ///
    /// A left click on a window's empty space raises it. The raise happens here, before the frame
    /// is rendered, which is why the click's own frame already draws the window on top.
    pub fn end_frame(&mut self) {
        // `ImGuiIO::InputQueueCharacters.resize(0)` in `EndFramePost`: the characters were consumed
        // by the widgets of this frame, so the next one starts empty. Done first because the rest
        // of this function returns early.
        self.io.input_queue_characters.clear();
        // `g.HoveredIdPreviousFrame = g.HoveredId`: the header the mouse is over now is the one the
        // next frame can report as hovered.
        self.header_hovered_last = self.header_hovered_next.take();
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
        content: Vec2,
        title_height: f32,
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
            if held && self.io.mouse_clicked_count[0] == 2 {
                // `CalcWindowAutoFitSize`, then the size constraint: the content plus padding, never
                // under the minimum. Double-clicking a grip fits the window to its content.
                let pad = self.style.window_padding;
                let minimum = if always_auto {
                    Vec2 { x: 1.0, y: 1.0 }
                } else {
                    self.style.window_min_size
                };
                let fitted = Vec2 {
                    x: (content.x + pad.x * 2.0).max(minimum.x),
                    y: (content.y + pad.y * 2.0 + title_height).max(minimum.y),
                };
                result = Some((pos, fitted));
            } else if held {
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
        self.resize_border_hovered = -1;
        self.resize_border_held = -1;
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
            if hovered {
                self.resize_border_hovered = border_n as i32;
            }
            if held {
                self.resize_border_held = border_n as i32;
            }
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

    /// `RenderWindowOuterSingleBorder`: the thicker stroke the reference draws over the border being
    /// grabbed, in the separator colours.
    fn draw_resize_border_highlight(&mut self, pos: Vec2, size: Vec2, border_n: i32, active: bool) {
        use crate::style::Col;

        let rounding = self.style.window_rounding;
        let border_size = self.style.window_border_size.max(2.0);
        let col = if active {
            self.style.color_u32(Col::SeparatorActive)
        } else {
            self.style.color_u32(Col::SeparatorHovered)
        };
        // `GetResizeBorderRect(window, border_n, rounding, 0.0f)`: a zero thickness also shrinks the
        // far corner by one, which the ports of the reference's rectangles inherit.
        let rect_min = pos;
        let rect_max = Vec2 {
            x: pos.x + size.x - 1.0,
            y: pos.y + size.y - 1.0,
        };
        let (n1, n2, angle) = match border_n {
            0 => (
                Vec2 { x: 0.0, y: 1.0 },
                Vec2 { x: 0.0, y: 0.0 },
                core::f32::consts::PI,
            ),
            1 => (Vec2 { x: 1.0, y: 0.0 }, Vec2 { x: 1.0, y: 1.0 }, 0.0),
            2 => (
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 1.0, y: 0.0 },
                core::f32::consts::PI * 1.5,
            ),
            _ => (
                Vec2 { x: 1.0, y: 1.0 },
                Vec2 { x: 0.0, y: 1.0 },
                core::f32::consts::PI * 0.5,
            ),
        };
        let inner_dir = match border_n {
            0 => Vec2 { x: 1.0, y: 0.0 },
            1 => Vec2 { x: -1.0, y: 0.0 },
            2 => Vec2 { x: 0.0, y: 1.0 },
            _ => Vec2 { x: 0.0, y: -1.0 },
        };
        let at = |n: Vec2| Vec2 {
            x: rect_min.x + (rect_max.x - rect_min.x) * n.x + 0.5 + inner_dir.x * rounding,
            y: rect_min.y + (rect_max.y - rect_min.y) * n.y + 0.5 + inner_dir.y * rounding,
        };
        // `PathArcTo` with a zero radius adds the point once, and `PathStroke` walks the two points.
        let _ = angle;
        let points = [at(n1), at(n2)];
        self.draw_list.add_polyline(&points, col, border_size, 0);
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

    /// The window being submitted, to change it while a function lays an item out.
    fn window_mut(&mut self) -> &mut Window {
        self.windows.last_mut().expect("no window is open")
    }

    /// `g.InputTextState`: the edit state of the field that owns the keyboard, if any.
    pub fn input_text_state(&self) -> Option<&crate::input_text::InputTextState> {
        self.input_text_state.as_ref()
    }

    /// `g.ActiveId`: the item that owns the mouse or the keyboard, if any.
    pub fn active_id(&self) -> Option<ImGuiID> {
        self.active_id
    }

    /// `ImGui::SetNextItemWidth`: the width the next item asks for.
    pub fn set_next_item_width(&mut self, width: f32) {
        self.next_item_width = Some(width);
    }

    /// `ImGui::CalcItemWidth`: the width the next item takes when it does not ask for one. The next
    /// item's own width wins, then the one `PushItemWidth` left on the stack (zero meaning the
    /// default), then the window's: 65 % of an explicitly sized window, sixteen font sizes otherwise.
    /// A negative width is that much less than the content available from the cursor.
    fn calc_item_width(&mut self) -> f32 {
        let width = match self.next_item_width.take() {
            Some(width) => width,
            None => match self.item_width_stack.last() {
                Some(&width) if width != 0.0 => width,
                _ => self.default_item_width(),
            },
        };
        if width < 0.0 {
            let window = self.window();
            let region_max = window.content_origin.x + window.content_size.x;
            return (region_max - window.cursor.x + width).max(1.0);
        }
        width.trunc()
    }

    /// `window->DC.ItemWidthDefault`: 65 % of an explicitly sized window, sixteen font sizes when the
    /// window sizes itself.
    fn default_item_width(&self) -> f32 {
        let window = self.window();
        let resizable =
            window.size.x > 0.0 && !window.flags.contains(WindowFlags::ALWAYS_AUTO_RESIZE);
        if resizable {
            (window.size.x * 0.65).trunc()
        } else {
            (self.font.size * 16.0).trunc()
        }
    }

    /// `ImGui::PushItemWidth`: the width later items take, remembered on a stack. Zero means the
    /// window's default.
    pub fn push_item_width(&mut self, width: f32) {
        self.item_width_stack.push(width);
    }

    /// `ImGui::PopItemWidth`: back to the width before the matching `PushItemWidth`.
    pub fn pop_item_width(&mut self) {
        self.item_width_stack.pop();
    }

    /// The whole of `ItemHoverable` as far as the mouse is concerned: the mouse is inside the
    /// rectangle *and* its window is the one under the pointer.
    fn hovered_in(&self, min: Vec2, max: Vec2) -> bool {
        let window = self.window();
        let own_window = self.hovered_window.as_deref() == Some(window.name.as_str());
        let Some(mouse) = self.io.mouse_pos else {
            return false;
        };
        own_window
            && mouse.x >= window.pos.x
            && mouse.x < window.pos.x + window.size.x
            && mouse.y >= window.pos.y
            && mouse.y < window.pos.y + window.size.y
            && mouse.x >= min.x
            && mouse.x < max.x
            && mouse.y >= min.y
            && mouse.y < max.y
    }

    /// `ImGui::PushStyleColor`: overrides one entry of the style colours until the matching pop.
    pub fn push_style_color(&mut self, col: crate::style::Col, colour: u32) {
        self.color_stack.push((col, self.style.colors[col.index()]));
        self.style.colors[col.index()] = colour;
    }

    /// `ImGui::PopStyleColor`: restores the colour the matching push replaced.
    pub fn pop_style_color(&mut self) {
        if let Some((col, previous)) = self.color_stack.pop() {
            self.style.colors[col.index()] = previous;
        }
    }

    /// `GetContentRegionAvail`: the content region from the cursor to its far corner, which is the
    /// `WorkRect.Max` of the reference minus `CursorPos`.
    pub fn content_avail(&self) -> Vec2 {
        let window = self.window();
        Vec2 {
            x: window.content_origin.x + window.content_size.x - window.cursor.x,
            y: window.content_origin.y + window.content_size.y - window.cursor.y,
        }
    }

    /// `GetCursorPos`: the cursor in window coordinates, which is what the layout functions use.
    pub fn cursor_pos(&self) -> Vec2 {
        let window = self.window();
        Vec2 {
            x: window.cursor.x - window.pos.x,
            y: window.cursor.y - window.pos.y,
        }
    }

    /// `SetCursorPos`: place the cursor at a window-relative position.
    pub fn set_cursor_pos(&mut self, pos: Vec2) {
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        window.cursor = Vec2 {
            x: window.pos.x + pos.x,
            y: window.pos.y + pos.y,
        };
    }

    /// `SetCursorPosX`: move only the cursor x, leaving the y where it is.
    pub fn set_cursor_pos_x(&mut self, local_x: f32) {
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        let y = window.cursor.y;
        window.cursor = Vec2 {
            x: window.pos.x + local_x,
            y,
        };
    }

    /// `SetCursorPosY`: move only the cursor y, leaving the x where it is.
    pub fn set_cursor_pos_y(&mut self, local_y: f32) {
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        let x = window.cursor.x;
        window.cursor = Vec2 {
            x,
            y: window.pos.y + local_y,
        };
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

    /// `GetTextLineHeight`: one line of text, which is the font size.
    pub fn text_line_height(&self) -> f32 {
        self.font.size
    }

    /// `GetTextLineHeightWithSpacing`: the line plus `ItemSpacing.y`.
    pub fn text_line_height_with_spacing(&self) -> f32 {
        self.font.size + self.style.item_spacing.y
    }

    /// `GetFrameHeight`: `FontSize + FramePadding.y * 2`, the height of a framed widget.
    pub fn get_frame_height(&self) -> f32 {
        self.frame_height()
    }

    /// `GetFrameHeightWithSpacing`: the frame plus `ItemSpacing.y`.
    pub fn frame_height_with_spacing(&self) -> f32 {
        self.frame_height() + self.style.item_spacing.y
    }
    /// `ImGui::Dummy`: an item of a given size that advances the cursor.
    /// `ImGui::OpenPopup`: remember that a popup is open. Its window is created by `begin_popup`.
    /// `ImGui::BeginCombo`: the framed box with its arrow. Returns true while its popup is open, and
    /// `end_combo` closes it.
    ///
    /// The box is one item: pressing it opens the popup. The popup is a window of its own, as wide as
    /// the box and laid out under it, falling back to the other three sides when it does not fit.
    pub fn begin_combo(&mut self, label: &str, preview: &str, width: f32) -> bool {
        let id = self.get_id(label);
        let pad = self.style.frame_padding;
        // `FindRenderedTextEnd`: the part after `##` identifies the widget but is not drawn.
        let shown = match label.find("##") {
            Some(at) => &label[..at],
            None => label,
        };
        let label_size = self.font.calc_text_size(shown, self.font.size);
        let arrow_size = self.frame_height();
        let bb_min = self.window().cursor;
        let bb_max = Vec2 {
            x: bb_min.x + width,
            y: bb_min.y + label_size.y + pad.y * 2.0,
        };
        let total = Vec2 {
            x: width
                + if label_size.x > 0.0 {
                    self.style.item_inner_spacing.x + label_size.x
                } else {
                    0.0
                },
            y: label_size.y + pad.y * 2.0,
        };
        let pressed = self.interact_rect_spanning(id, bb_min, bb_max, total, false);
        let hovered = self.last_item_hovered;
        let popup_id = hash_str_utf8("##ComboPopup", id);
        // A row chosen last frame closes the popup now, so that frame still drew it.
        if self.popup_pending_close {
            self.popup_open = None;
            self.popup_pending_close = false;
        }
        let mut open = self.popup_open == Some(popup_id);
        if pressed && !open {
            self.popup_open = Some(popup_id);
            open = true;
        }

        // Render shape: the preview's frame, then the arrow's own button on the right.
        let value_x2 = (bb_max.x - arrow_size).max(bb_min.x);
        let frame_col = if hovered {
            self.style.color_u32(crate::style::Col::FrameBgHovered)
        } else {
            self.style.color_u32(crate::style::Col::FrameBg)
        };
        self.draw_list.prim_rect(
            bb_min,
            Vec2 {
                x: value_x2,
                y: bb_max.y,
            },
            frame_col,
        );
        let arrow_bg = if open || hovered {
            self.style.color_u32(crate::style::Col::ButtonHovered)
        } else {
            self.style.color_u32(crate::style::Col::Button)
        };
        self.draw_list.prim_rect(
            Vec2 {
                x: value_x2,
                y: bb_min.y,
            },
            bb_max,
            arrow_bg,
        );
        let text_col = self.style.color_u32(crate::style::Col::Text);
        self.draw_list.add_arrow(
            Vec2 {
                x: value_x2 + pad.y,
                y: bb_min.y + pad.y,
            },
            text_col,
            ARROW_DOWN_COS,
            ARROW_DOWN_SIN,
            1.0,
            self.font.size,
            self.style.symbol_arrow_size_factor,
            self.style.symbol_arrow_thickness_factor,
        );
        if self.style.frame_border_size > 0.0 {
            self.draw_list.add_aligned_rect_border(
                bb_min,
                bb_max,
                self.style.color_u32(crate::style::Col::Border),
                self.style.frame_border_size,
            );
        }
        let font = self.font.clone();
        self.draw_list.add_text(
            &font,
            font.size,
            Vec2 {
                x: bb_min.x + pad.x,
                y: bb_min.y + pad.y,
            },
            text_col,
            preview,
        );
        if label_size.x > 0.0 {
            self.draw_list.add_text(
                &font,
                font.size,
                Vec2 {
                    x: bb_max.x + self.style.item_inner_spacing.x,
                    y: bb_min.y + pad.y,
                },
                text_col,
                shown,
            );
        }
        if !open {
            return false;
        }
        self.begin_combo_popup(bb_min, bb_max)
    }

    /// `BeginComboPopup`: the popup window, laid out under the box and toward the right.
    fn begin_combo_popup(&mut self, bb_min: Vec2, bb_max: Vec2) -> bool {
        // `ImFormatString("##Combo_%02d")`: the window is recycled by depth.
        let name = "##Combo_00";
        let measured = WindowMemory::find(&self.memory, name).map(|memory| memory.size_full);
        let pos = combo_popup_pos(
            (Vec2::default(), self.io.display_size),
            (bb_min, bb_max),
            measured.unwrap_or_default(),
        );
        self.popup_first_frame = measured.is_none();
        // `SetNextWindowSizeConstraints(constraint_min = (width, 0), ...)`: at least as wide as the box.
        self.next_size_min = Some(Vec2 {
            x: bb_max.x - bb_min.x,
            y: 0.0,
        });
        let flags = WindowFlags::ALWAYS_AUTO_RESIZE
            | WindowFlags::NO_RESIZE
            | WindowFlags::NO_MOVE
            | WindowFlags::NO_SCROLLBAR
            | WindowFlags::POPUP;
        self.set_next_window_flags(flags);
        // The popup belongs to the window that is current now, and owns a list of its own.
        self.popup_host = self.windows.last().map(|window| window.name.clone());
        let host = core::mem::take(&mut self.draw_list);
        self.list_stack.push(host);
        // `PushStyleVarX(WindowPadding, FramePadding.x)`: the rows line up with the framed preview.
        self.combo_saved_padding = Some(self.style.window_padding.x);
        self.style.window_padding.x = self.style.frame_padding.x;
        self.last_popup_name = Some(name.to_string());
        self.begin_window(name, Some(pos), None);
        true
    }

    /// `ImGui::EndCombo`: close the popup's window and hand the host its list back.
    pub fn end_combo(&mut self) {
        let (pos, size) = {
            let window = self.window();
            (window.pos, window.size)
        };
        self.end_window();
        if self.popup_first_frame {
            // Nothing was drawn: drop the list the way the reference skips an empty one.
            self.frame_lists.pop();
            self.list_parents.pop();
            self.popup_first_frame = false;
        }
        self.draw_list = self
            .list_stack
            .pop()
            .unwrap_or_else(|| self.new_window_draw_list());
        if let Some(saved) = self.combo_saved_padding.take() {
            self.style.window_padding.x = saved;
        }
        self.popup_rect = self
            .last_popup_name
            .take()
            .map(|name| (name, (pos, size)))
            .or(self.popup_rect.take());
    }

    pub fn open_popup(&mut self, id: &str) {
        self.popup_open = Some(self.get_id(id));
    }

    /// `ImGui::BeginPopup`: the popup's window, anchored where the caller asks for.
    ///
    /// A popup owns a draw list of its own, which is why the window hosting it keeps its list aside
    /// while the popup is open. On its first frame a popup only measures its content: the list it
    /// produced is empty in the reference and never reaches the draw data.
    pub fn begin_popup(&mut self, id: &str, pos: Option<Vec2>) -> bool {
        let popup_id = self.get_id(id);
        // A click outside the popup closed it last frame; that frame still drew it.
        if self.popup_pending_close {
            self.popup_open = None;
            self.popup_pending_close = false;
        }
        if self.popup_open != Some(popup_id) {
            return false;
        }
        // `ImFormatString("%08x")`: the reference writes the popup id in lower case.
        let name = format!("##Popup_{popup_id:08x}");
        let measured = WindowMemory::find(&self.memory, &name).map(|memory| memory.size_full);
        self.popup_first_frame = measured.is_none();
        let flags = WindowFlags::ALWAYS_AUTO_RESIZE
            | WindowFlags::NO_RESIZE
            | WindowFlags::NO_MOVE
            | WindowFlags::NO_SCROLLBAR
            | WindowFlags::POPUP;
        // The popup belongs to the window that is current now: it takes the focus back when it closes.
        self.popup_host = self.windows.last().map(|window| window.name.clone());
        self.set_next_window_flags(flags);
        let host = core::mem::take(&mut self.draw_list);
        self.list_stack.push(host);
        self.last_popup_name = Some(name.clone());
        self.begin_window(&name, pos, None);
        true
    }

    /// `ImGui::EndPopup`: close the popup's window, and the popup itself when a click landed outside
    /// it. The frame that closes it still draws it, so the close is applied on the next `begin_popup`.
    pub fn end_popup(&mut self) {
        let (pos, size) = {
            let window = self.window();
            (window.pos, window.size)
        };
        self.end_window();
        if self.popup_first_frame {
            // Nothing was drawn: drop the list the way the reference skips an empty one.
            self.frame_lists.pop();
            self.list_parents.pop();
            self.popup_first_frame = false;
        }
        self.draw_list = self
            .list_stack
            .pop()
            .unwrap_or_else(|| self.new_window_draw_list());
        // The popup sits above every window, so the next frame's hover search needs its rectangle.
        let name = self.last_popup_name.clone();
        if let Some(name) = name {
            self.popup_rect = Some((name, (pos, size)));
        }
    }

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
        // `window->DC.LayoutType == Horizontal`: a menu bar lays its items out in a row, so the cursor
        // moves to the right of the item instead of below the line.
        if window.layout_horizontal {
            window.cursor = Vec2 {
                x: max.x + spacing.x,
                y: min.y,
            };
            return;
        }
        // The line ends here: the cursor moves below it and back to the line start.
        window.cursor = Vec2 {
            x: window.line_start_x,
            y: min.y + window.line_height + spacing.y,
        };
    }

    /// `ImGui::CalcItemSize`: the size an item takes from the `size_arg` its widget asks for. Zero
    /// means the default width or height, a negative axis fills up to the content edge (what the
    /// `ImVec2(-FLT_MIN, 0)` default of widgets like `ProgressBar` asks for) and a positive one is
    /// taken as is.
    fn calc_item_size(&mut self, size: Vec2, default_w: f32, default_h: f32) -> Vec2 {
        let (cursor, region_max) = {
            let window = self.window();
            (
                window.cursor,
                Vec2 {
                    x: window.content_origin.x + window.content_size.x,
                    y: window.content_origin.y + window.content_size.y,
                },
            )
        };
        let axis = |value: f32, default: f32, region_max: f32, cursor: f32| {
            if value == 0.0 {
                default
            } else if value < 0.0 {
                (region_max - cursor + value).max(4.0)
            } else {
                value
            }
        };
        Vec2 {
            x: axis(size.x, default_w, region_max.x, cursor.x),
            y: axis(size.y, default_h, region_max.y, cursor.y),
        }
    }

    /// `ImGui::ProgressBar`: a frame filled up to `fraction` with `ImGuiCol_PlotHistogram` and the
    /// overlay (by default the percentage) just after the fill, clipped to the bar. A negative
    /// fraction draws the indeterminate slice the caller animates.
    pub fn progress_bar(&mut self, fraction: f32, size: Option<Vec2>, overlay: Option<&str>) {
        if self
            .windows
            .last()
            .map(|window| window.collapsed)
            .unwrap_or(false)
        {
            return;
        }
        let frame_padding = self.style.frame_padding;
        let font_size = self.font.size;
        let default_width = self.calc_item_width();
        // The reference's default `size_arg` is `ImVec2(-FLT_MIN, 0)`: as wide as the content area.
        let size_arg = size.unwrap_or(Vec2 {
            x: -f32::MIN_POSITIVE,
            y: 0.0,
        });
        let size = self.calc_item_size(size_arg, default_width, font_size + frame_padding.y * 2.0);
        let pos = self.window().cursor;
        let bb = (
            pos,
            Vec2 {
                x: pos.x + size.x,
                y: pos.y + size.y,
            },
        );
        // `ItemSize(size, FramePadding.y)` then `ItemAdd(bb, 0)`: the whole bar is the item.
        self.dummy(size);
        // A fraction below zero is the indeterminate bar, whose slice the caller animates with time.
        let indeterminate = fraction < 0.0;
        let clamped = if indeterminate {
            fraction
        } else {
            fraction.clamp(0.0, 1.0)
        };
        // A NaN fraction is tolerated as an empty bar, like the reference does.
        let (mut fill_n0, mut fill_n1) = (0.0f32, if clamped.is_nan() { 0.0 } else { clamped });
        if indeterminate {
            let fill_width_n = 0.2f32;
            fill_n0 = (-fraction % 1.0) * (1.0 + fill_width_n) - fill_width_n;
            fill_n1 = (fill_n0 + fill_width_n).clamp(0.0, 1.0);
            fill_n0 = fill_n0.clamp(0.0, 1.0);
        }
        // `RenderFrame` with `FrameRounding` 0 and no border is a plain filled rectangle.
        let frame_colour = self.style.color_u32(crate::style::Col::FrameBg);
        self.draw_list.add_rect_filled(bb.0, bb.1, frame_colour);
        // ImLerp between the bar's edges: where the filled part starts and ends.
        let fill_x0 = bb.0.x + (bb.1.x - bb.0.x) * fill_n0;
        let fill_x1 = bb.0.x + (bb.1.x - bb.0.x) * fill_n1;
        if fill_x0 < fill_x1 {
            let fill_colour = self.style.color_u32(crate::style::Col::PlotHistogram);
            self.draw_list.add_rect_filled(
                Vec2 {
                    x: fill_x0,
                    y: bb.0.y,
                },
                Vec2 {
                    x: fill_x1,
                    y: bb.1.y,
                },
                fill_colour,
            );
        }
        // The default overlay is the percentage, and an indeterminate bar shows none unless asked.
        if !indeterminate || overlay.is_some() {
            // `FindRenderedTextEnd`: everything after "##" identifies the overlay but is not drawn.
            let text = match overlay {
                Some(text) => rendered_text(text).to_string(),
                None => format!("{:.0}%", clamped * 100.0 + 0.01),
            };
            let text_size = self.font.calc_text_size(&text, font_size);
            if text_size.x > 0.0 {
                let text_x = if indeterminate {
                    (bb.0.x + bb.1.x - text_size.x) * 0.5
                } else {
                    fill_x1 + self.style.item_spacing.x
                };
                let text_x = text_x.clamp(
                    bb.0.x,
                    bb.1.x - text_size.x - self.style.item_inner_spacing.x,
                );
                // `RenderTextClipped` with `align (0, 0.5)` and the bar as its clip rectangle.
                let text_pos = Vec2 {
                    x: text_x,
                    y: bb.0.y + ((bb.1.y - bb.0.y - text_size.y) * 0.5).trunc(),
                };
                let colour = self.style.color_u32(crate::style::Col::Text);
                self.draw_list.add_text_in(
                    &self.font.clone(),
                    font_size,
                    text_pos,
                    colour,
                    &text,
                    Some([bb.0.x, bb.0.y, bb.1.x, bb.1.y]),
                    None,
                );
            }
        }
        if let Some(window) = self.windows.last_mut() {
            window.last_min = bb.0;
            window.last_max = bb.1;
            window.has_item = true;
        }
    }

    /// `ImGui::BulletText`: one item that holds a disc followed by the text, so the entry is indented
    /// by the bullet and the text starts at `FontSize + FramePadding.x * 2` from the item's left.
    pub fn bullet_text(&mut self, text: &str) {
        if self
            .windows
            .last()
            .map(|window| window.collapsed)
            .unwrap_or(false)
        {
            return;
        }
        let font_size = self.font.size;
        let frame_padding = self.style.frame_padding;
        let font = self.font.clone();
        let label_size = font.calc_text_size(text, font.size);
        // `ImVec2(FontSize + (label > 0 ? label.x + FramePadding.x * 2 : 0), label.y)`: empty text adds
        // no padding.
        let total = Vec2 {
            x: font_size
                + if label_size.x > 0.0 {
                    label_size.x + frame_padding.x * 2.0
                } else {
                    0.0
                },
            y: label_size.y,
        };
        let pos = self.window().cursor;
        self.dummy(total);
        if let Some(window) = self.windows.last_mut() {
            window.last_min = pos;
            window.last_max = Vec2 {
                x: pos.x + total.x,
                y: pos.y + total.y,
            };
            window.has_item = true;
        }
        let colour = self.style.color_u32(crate::style::Col::Text);
        // `RenderBullet` at `bb.Min + (FramePadding.x + FontSize * 0.5, FontSize * 0.5)`.
        self.draw_list.add_circle_filled(
            Vec2 {
                x: pos.x + frame_padding.x + font_size * 0.5,
                y: pos.y + font_size * 0.5,
            },
            font_size * 0.20,
            colour,
            8,
        );
        let clip = self.window().clip_rect;
        self.draw_list.add_text_in(
            &font,
            font.size,
            Vec2 {
                x: pos.x + font_size + frame_padding.x * 2.0,
                y: pos.y,
            },
            colour,
            text,
            clip,
            None,
        );
    }

    /// `ImGui::NewLine`: ends the line by reserving one font height, so the cursor moves below it.
    ///
    /// The reference branches on `CurrLineSize.y`, which every `ItemSize` resets, so the branch only
    /// differs on a line whose height was set by hand (`AlignTextToFramePadding`, i.e. a menu bar),
    /// which no golden covers yet.
    pub fn new_line(&mut self) {
        let font_size = self.font.size;
        self.dummy(Vec2 {
            x: 0.0,
            y: font_size,
        });
    }

    /// `ImGui::Spacing`: an empty item of no size, which closes the current line and leaves the gap
    /// of `ItemSpacing.y` below it.
    pub fn spacing(&mut self) {
        self.dummy(Vec2 { x: 0.0, y: 0.0 });
    }

    /// `ImGui::Bullet`: a disc of radius `FontSize * 0.2` at the start of a line, with the next item
    /// kept on that same line, which is what makes it indent a list entry.
    pub fn bullet(&mut self) {
        let font_size = self.font.size;
        let frame_padding = self.style.frame_padding;
        let Some(window) = self.windows.last() else {
            return;
        };
        if window.collapsed {
            return;
        }
        // `ImMax(ImMin(CurrLineSize.y, FontSize + FramePadding.y * 2), FontSize)`: the bullet is as
        // tall as the line it lands on, but never smaller than the font.
        let line_height = window
            .line_height
            .min(font_size + frame_padding.y * 2.0)
            .max(font_size);
        let pos = window.cursor;
        let bb = (
            pos,
            Vec2 {
                x: pos.x + font_size,
                y: pos.y + line_height,
            },
        );
        // `ItemSize(bb)` then `ItemAdd(bb, 0)`: the item is a square as wide as the font.
        self.dummy(Vec2 {
            x: font_size,
            y: line_height,
        });
        let colour = self.style.color_u32(crate::style::Col::Text);
        // `RenderBullet`: an eight-segment disc at `bb.Min + (FramePadding.x + FontSize * 0.5,
        // line_height * 0.5)`.
        self.draw_list.add_circle_filled(
            Vec2 {
                x: bb.0.x + frame_padding.x + font_size * 0.5,
                y: bb.0.y + line_height * 0.5,
            },
            font_size * 0.20,
            colour,
            8,
        );
        if let Some(window) = self.windows.last_mut() {
            window.last_min = bb.0;
            window.last_max = bb.1;
            window.has_item = true;
        }
        // `SameLine(0, FramePadding.x * 2)`: the next item shares the bullet's line.
        let spacing = Vec2 {
            x: frame_padding.x * 2.0,
            y: self.style.item_spacing.y,
        };
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        window.cursor = Vec2 {
            x: window.last_max.x + spacing.x,
            y: window.last_min.y,
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

    /// `ImGui::BeginTable`: reserves the table's rectangle at the cursor, pushes its identifier and
    /// reports whether the table can be used.
    ///
    /// `outer_size` of zero means "as wide as the content area and no height", the only shape this
    /// first cut lays out; the columns are then sized by `table_setup_column`.
    pub fn begin_table(&mut self, id: &str, columns: usize, flags: u32, outer_size: Vec2) -> bool {
        let Some(window) = self.windows.last() else {
            return false;
        };
        if window.collapsed || columns == 0 {
            return false;
        }
        // `CalcItemSize(outer_size, max(avail.x, hard_min), 0)`, then truncated like the reference.
        let avail = window.content_size;
        let origin = window.cursor;
        let line_start_x = window.line_start_x;
        let width = if outer_size.x == 0.0 {
            avail.x.max(1.0)
        } else {
            outer_size.x
        };
        let height = if outer_size.y == 0.0 {
            0.0
        } else {
            outer_size.y
        };
        let outer_rect = (
            origin,
            Vec2 {
                x: origin.x + width.trunc(),
                y: origin.y + height.trunc(),
            },
        );
        let host_clip = self.draw_list.current_clip_rect();
        // `BeginTableEx`: a table with `ScrollX`/`ScrollY` builds its rows in an inner child window
        // with no padding and no border, so the content scrolls and clips inside the table's
        // rectangle. The host's list keeps the item; the child gets a list of its own.
        let use_child = flags
            & (crate::tables::table_flags::SCROLL_X | crate::tables::table_flags::SCROLL_Y)
            != 0;
        if use_child {
            self.begin_table_child(id, outer_rect, host_clip);
            // `TableUpdateLayout` pushes `inner_window->InnerClipRect`, which for an inner window is
            // the clip the child already carries: same rectangle, no command split.
            let clip = self.draw_list.current_clip_rect();
            self.draw_list.push_clip_rect(
                Vec2 {
                    x: clip[0],
                    y: clip[1],
                },
                Vec2 {
                    x: clip[2],
                    y: clip[3],
                },
                false,
            );
        } else {
            // `BeginTableEx` pushes the window's inner clip on the list. Inside a plain window that is
            // the clip the list is already on, so this neither changes nor splits a command.
            self.draw_list.push_clip_rect(
                Vec2 {
                    x: host_clip[0],
                    y: host_clip[1],
                },
                Vec2 {
                    x: host_clip[2],
                    y: host_clip[3],
                },
                false,
            );
        }
        self.push_id(id);
        self.table_host = Some(TableHost { line_start_x });
        self.table_outer_clip = Some(host_clip);
        self.table_indices = vec![Vec::new(); columns];
        self.table_bg_indices.clear();
        let mut table = crate::tables::Table::new(flags, columns, outer_rect);
        // The instance data of a table outlives the frame that built it.
        if let Some(&next) = self.table_hovered_row_next.get(id) {
            table.hovered_row_next = next;
        }
        // `TableLoadSettings`: the sort state the table ended the previous frame with (the reference
        // keeps it in the table it reuses and reloads it from its `.ini`), so the `DefaultSort` flags
        // only pick the starting state of a table that has none. The specs start dirty on the first
        // frame, which is what tells the caller to sort.
        table.sort_specs_dirty = true;
        if let Some(state) = self.table_sort_state.get(id) {
            table.restore_sort_state(state);
        }
        // `TableSaveSettings` for the display order, then the reorder request the previous frame's
        // drag queued, which `BeginTableEx` applies before the layout runs.
        if let Some((orders, reorder_column, reorder_dst)) = self.table_column_order.get(id) {
            if orders.len() == columns {
                table.display_orders.clone_from(orders);
                table.reorder_column = *reorder_column;
                table.reorder_column_dst_order = *reorder_dst;
                table.fix_display_order();
            }
        }
        table.apply_pending_reorder();
        // `if (g.ActiveId == 0) table->ReorderColumn = -1`: the header no longer holds the drag, so
        // the next click on it sorts again. `header_active` is the port's ActiveId for headers.
        if self.header_active.is_none() {
            table.reorder_column = -1;
        }
        self.table = Some(table);
        self.table_id = Some(id.to_string());
        true
    }

    /// `ImGui::TableSetupColumn`: declares the next column, in order, before the first row.
    ///
    /// The `label` is the column name the headers draw.
    pub fn table_setup_column(&mut self, label: &str, flags: u32, width: f32) {
        let extents = self
            .table_id
            .as_ref()
            .and_then(|id| self.table_content_max_x.get(id))
            .cloned();
        if let Some(table) = self.table.as_mut() {
            let index = table.decl_columns;
            table.setup_column(label, flags, width);
            if let Some(column) = table.columns.get_mut(index) {
                if let Some(extent) = extents.and_then(|e| e.get(index).copied()) {
                    column.content_width = extent;
                }
            }
        }
    }

    /// `ImGui::TableNextRow`: starts a row whose height follows its contents.
    pub fn table_next_row(&mut self) {
        self.table_next_row_min(0.0);
    }

    /// `ImGui::TableNextRow` with an explicit minimum height.
    pub fn table_next_row_min(&mut self, row_min_height: f32) {
        self.table_next_row_flags(0, row_min_height);
    }

    /// `ImGui::TableNextRow` with row flags, which is how a header row marks itself.
    pub fn table_next_row_flags(&mut self, row_flags: u32, row_min_height: f32) {
        let Some(mut table) = self.table.take() else {
            return;
        };
        self.table_lock_layout(&mut table);
        if table.is_inside_row {
            self.table_end_row(&mut table);
        }
        table.row_cell_padding_y = self.style.cell_padding.y;
        // The row background: a header row always has its own, `RowBg` alternates over the data
        // rows, and an explicit colour would win over both.
        let row_bg = if row_flags & crate::tables::table_flags::ROW_HEADERS != 0 {
            Some(self.style.table_header_bg)
        } else if table.flags & crate::tables::table_flags::ROW_BG != 0 {
            Some(if table.row_bg_counter & 1 != 0 {
                self.style.table_row_bg_alt
            } else {
                self.style.table_row_bg
            })
        } else {
            None
        };
        table.begin_row(row_min_height, row_flags, row_bg);
        // `TableBeginRow` restarts the row's own extent at its top.
        if let Some(window) = self.windows.last_mut() {
            window.cursor_max.y = table.row_pos_y1;
        }
        self.table = Some(table);
    }

    /// `ImGui::TableNextColumn`: the next cell, wrapping into a new row after the last column.
    pub fn table_next_column(&mut self) -> bool {
        let Some(mut table) = self.table.take() else {
            return false;
        };
        self.table_lock_layout(&mut table);
        let next = table.current_column + 1;
        if table.is_inside_row && next < table.columns.len() as i32 {
            if table.current_column != -1 {
                self.table_end_cell(&mut table);
            }
            self.table_begin_cell(&mut table, next as usize);
        } else {
            // `TableNextRow()` then the first cell of it.
            if table.is_inside_row {
                self.table_end_row(&mut table);
            }
            table.row_cell_padding_y = self.style.cell_padding.y;
            let row_bg = if table.flags & crate::tables::table_flags::ROW_BG != 0 {
                Some(if table.row_bg_counter & 1 != 0 {
                    self.style.table_row_bg_alt
                } else {
                    self.style.table_row_bg
                })
            } else {
                None
            };
            table.begin_row(0.0, 0, row_bg);
            if let Some(window) = self.windows.last_mut() {
                window.cursor_max.y = table.row_pos_y1;
            }
            self.table_begin_cell(&mut table, 0);
        }
        self.table = Some(table);
        true
    }

    /// `ImGui::TableSetColumnIndex`: moves to a specific column, closing the current cell first.
    pub fn table_set_column_index(&mut self, column: usize) -> bool {
        let Some(mut table) = self.table.take() else {
            return false;
        };
        self.table_lock_layout(&mut table);
        if column >= table.columns.len() {
            self.table = Some(table);
            return false;
        }
        if table.current_column != column as i32 {
            if table.current_column != -1 {
                self.table_end_cell(&mut table);
            }
            self.table_begin_cell(&mut table, column);
        }
        self.table = Some(table);
        true
    }

    /// `ImGui::TableHeadersRow`: one header row holding the declared column names.
    ///
    /// The row is as tall as the tallest name plus two cell paddings, it paints `TableHeaderBg`
    /// across the whole table width, and it counts for [`Self::table_get_row_index`].
    pub fn table_headers_row(&mut self) {
        let Some(mut table) = self.table.take() else {
            return;
        };
        self.table_lock_layout(&mut table);
        let mut row_height = self.font.size;
        for column in &table.columns {
            row_height = row_height.max(self.font.calc_text_size(&column.name, self.font.size).y);
        }
        row_height += self.style.cell_padding.y * 2.0;
        let names: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();
        self.table = Some(table);
        self.table_next_row_flags(crate::tables::table_flags::ROW_HEADERS, row_height);
        for (n, name) in names.iter().enumerate() {
            if !self.table_set_column_index(n) {
                continue;
            }
            self.table_header(name);
        }
    }

    /// `ImGui::TableHeader`: the label of one header cell. Its item is the whole cell, not the text
    /// box, which is what `GetItemRect` reports while the label is being submitted.
    ///
    /// A sortable column reserves the right end of the cell for the sort arrow and draws it, with the
    /// sort order number when the spec is not the first one. Hover, the click that changes the specs
    /// and the reorder drag are not ported yet.
    pub fn table_header(&mut self, label: &str) {
        let (
            cell,
            row_min_height,
            row_padding_y,
            column_n,
            sortable,
            no_sort,
            sort_order,
            sort_direction,
            reorderable,
        ) = {
            let Some(table) = self.table.as_ref() else {
                return;
            };
            if table.current_column < 0 {
                return;
            }
            let column_n = table.current_column as usize;
            let column_flags = table
                .columns
                .get(column_n)
                .map(|column| column.flags)
                .unwrap_or(0);
            (
                table.cell_rect(column_n),
                table.row_min_height,
                table.row_cell_padding_y,
                column_n,
                table.flags & crate::tables::table_flags::SORTABLE != 0,
                column_flags & crate::tables::column_flags::NO_SORT != 0,
                table.sort_order(column_n),
                table.sort_direction(column_n),
                table.flags & crate::tables::table_flags::REORDERABLE != 0,
            )
        };
        // `FindRenderedTextEnd`: everything after "##" identifies the column but is not drawn.
        let shown = match label.find("##") {
            Some(at) => &label[..at],
            None => label,
        };
        let font = self.font.clone();
        let label_pos = self.window().cursor;
        let label_size = font.calc_text_size(shown, font.size);
        // A label is as tall as its text, or as the row minus its padding, whichever is more.
        let label_height = label_size.y.max(row_min_height - row_padding_y * 2.0);
        let bb = (
            cell.0,
            Vec2 {
                x: cell.1.x,
                y: cell
                    .1
                    .y
                    .max(cell.0.y + label_height + self.style.cell_padding.y * 2.0),
            },
        );
        // `TableHeader`'s ideal sort arrow: every sortable column reserves it, whether or not it is
        // the sorted one, and a spec that is not the first one also gets its order number.
        const ARROW_SCALE: f32 = 0.65;
        let mut w_arrow = 0.0f32;
        let mut w_sort_text = 0.0f32;
        let mut sort_arrow = false;
        let mut sort_order_suf = String::new();
        if sortable && !no_sort {
            w_arrow = (font.size * ARROW_SCALE + self.style.frame_padding.x).trunc();
            if sort_order != -1 {
                sort_arrow = true;
            }
            if sort_order > 0 {
                sort_order_suf = (sort_order + 1).to_string();
                w_sort_text = self.style.item_inner_spacing.x
                    + font.calc_text_size(&sort_order_suf, font.size).x;
            }
        }
        // `ellipsis_max`: the label is clipped where the arrow starts. The clipping itself is not
        // modelled yet, because no golden reaches a label that long.
        let _ellipsis_max = (cell.1.x
            - if sort_arrow {
                w_arrow + w_sort_text
            } else {
                0.0
            })
        .max(label_pos.x);
        // `TableHeader` hover: the header is an `AllowOverlap` item, so it only reports hover when it
        // already was the hovered item on the frame before; the mouse over it latches the id for the
        // next frame either way.
        let key = self.table_id.clone().map(|id| (id, column_n));
        let over = self.hovered_in(bb.0, bb.1);
        if let (true, Some(key)) = (over, key.clone()) {
            self.header_hovered_next = Some(key);
        }
        let hovered = over && key.is_some() && self.header_hovered_last == key;
        // `ButtonBehavior`: pressing a hovered header takes `ActiveId` until the button is released,
        // and the release over the item is the press that changes the sort order. Only the header
        // that holds the active id consumes the release: the ones before it in the row must leave it
        // alone, or a click on the second column would be swallowed by the first.
        let mut pressed = false;
        if self.io.mouse_down[0] {
            if hovered {
                self.header_active = key.clone();
            }
        } else if key.is_some() && self.header_active == key {
            self.header_active = None;
            pressed = hovered;
        }
        let held = key.is_some() && self.header_active == key;
        // A hovered cell takes `HeaderHovered`; a header outside a header row still paints its cell,
        // or the row would look plain where `TableHeadersRow` painted the others.
        let not_a_header_row = self
            .table
            .as_ref()
            .map(|table| table.row_flags & crate::tables::table_flags::ROW_HEADERS == 0)
            .unwrap_or(false);
        let bg = if held {
            Some(self.style.header_active)
        } else if hovered {
            Some(self.style.header_hovered)
        } else if not_a_header_row {
            Some(self.style.table_header_bg)
        } else {
            None
        };
        if let Some(bg) = bg {
            self.table_set_cell_bg_color(bg);
        }
        // The reorder drag: while the pressed header is held and the mouse crosses past its edge, the
        // column asks for the display position of the neighbour it crossed. The move is applied by the
        // next `BeginTable`, as the reference queues it, so the layout never changes mid-frame.
        if held && reorderable && self.io.is_mouse_dragging(0, -1.0) {
            if let Some(mouse) = self.io.mouse_pos {
                let delta_x = self.io.mouse_delta.x;
                if let Some(mut table) = self.table.take() {
                    let current = table.display_order(column_n);
                    if delta_x < 0.0 && mouse.x < cell.0.x {
                        let previous = table.column_at_display_order(current - 1);
                        if previous != -1 {
                            let target = table.display_order(previous as usize);
                            table.queue_column_display_order(column_n, target);
                        }
                    }
                    if delta_x > 0.0 && mouse.x > cell.1.x {
                        let next = table.column_at_display_order(current + 1);
                        if next != -1 {
                            let target = table.display_order(next as usize);
                            table.queue_column_display_order(column_n, target);
                        }
                    }
                    self.table = Some(table);
                }
            }
        }
        // The sort order number, faded to 70 %, and the arrow, both before the label.
        if sort_arrow {
            let mut x = cell.0.x.max(cell.1.x - w_arrow - w_sort_text);
            let y = label_pos.y;
            if sort_order > 0 {
                let faded = self.style.color_u32_alpha(crate::style::Col::Text, 0.70);
                self.draw_list.add_text(
                    &font,
                    font.size,
                    Vec2 {
                        x: x + self.style.item_inner_spacing.x,
                        y,
                    },
                    faded,
                    &sort_order_suf,
                );
                x += w_sort_text;
            }
            let (cos_a, sin_a) = if sort_direction == 2 {
                (ARROW_DOWN_COS, ARROW_DOWN_SIN)
            } else {
                (ARROW_UP_COS, ARROW_UP_SIN)
            };
            self.draw_list.add_arrow(
                Vec2 {
                    x: x + font.size * (1.0 - ARROW_SCALE) * 0.5,
                    y,
                },
                self.style.color_u32(crate::style::Col::Text),
                cos_a,
                sin_a,
                ARROW_SCALE,
                font.size,
                self.style.symbol_arrow_size_factor,
                self.style.symbol_arrow_thickness_factor,
            );
        }
        // `Handle clicking on column header to adjust Sort Order`, after the arrow was drawn: the
        // direction the arrow shows changes on the next frame. Shift appends the column to the specs
        // when the table allows more than one.
        if pressed && sortable && !no_sort {
            let shift = self.io.key_shift;
            if let Some(mut table) = self.table.take() {
                // `pressed && table->ReorderColumn != column_n`: a header that was just dragged does
                // not also apply a click.
                if table.reorder_column != column_n as i32 {
                    let next = table.next_sort_direction(column_n);
                    table.set_column_sort_direction(column_n, next, shift);
                }
                self.table = Some(table);
            }
        }
        let colour = self.style.color_u32(crate::style::Col::Text);
        self.draw_list
            .add_text(&font, font.size, label_pos, colour, shown);
        self.dummy(Vec2 {
            x: 0.0,
            y: label_height,
        });
        if let Some(window) = self.windows.last_mut() {
            window.last_min = bb.0;
            window.last_max = bb.1;
            window.has_item = true;
            // `window->DC.CursorPos.y -= ItemSpacing.y * 0.5f`.
            window.cursor.y -= self.style.item_spacing.y * 0.5;
        }
    }

    /// `ImGui::TableSetBgColor` for the current cell: the fill is collected now and painted when the
    /// row closes, because the row height is not known yet.
    pub fn table_set_cell_bg_color(&mut self, color: u32) {
        if let Some(table) = self.table.as_mut() {
            table.set_cell_bg(color);
        }
    }

    /// `ImGui::TableGetColumnCount`, and 0 when no table is open.
    pub fn table_get_column_count(&self) -> usize {
        self.table.as_ref().map(|t| t.columns.len()).unwrap_or(0)
    }

    /// `ImGui::TableGetSortSpecs`: the columns to sort by and whether they changed, or `None` when the
    /// table is not sortable. Reading them clears the internal dirty flag, as the reference does; the
    /// flag it hands back is `SpecsDirty`, which stays set until the caller clears it.
    ///
    /// The reference locks the layout first, because building the specs is what linearizes the sort
    /// orders; that also makes the call usable before the first row.
    pub fn table_get_sort_specs(&mut self) -> Option<(Vec<(usize, i32)>, bool)> {
        self.table.as_ref()?;
        if let Some(mut table) = self.table.take() {
            if table.flags & crate::tables::table_flags::SORTABLE != 0 {
                self.table_lock_layout(&mut table);
                // `TableSortSpecsBuild`: a click during the header row changed the orders after the
                // layout locked, so the specs are rebuilt here before they are handed out.
                table.build_sort_specs();
            }
            self.table = Some(table);
        }
        let table = self.table.as_mut()?;
        if table.flags & crate::tables::table_flags::SORTABLE == 0 {
            return None;
        }
        let specs = table.sort_specs.clone();
        let dirty = table.sort_specs_public_dirty;
        table.sort_specs_dirty = false;
        Some((specs, dirty))
    }

    /// Writing `specs->SpecsDirty = false` after sorting: only the flag the caller reads is cleared,
    /// because the specs themselves are already up to date.
    pub fn table_clear_sort_specs_dirty(&mut self) {
        if let Some(table) = self.table.as_mut() {
            table.sort_specs_public_dirty = false;
        }
    }

    /// `ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0)`: the colour of the current row, which
    /// the public call can set without the `RowBg` flag.
    pub fn table_set_row_bg_color(&mut self, color: u32) {
        if let Some(table) = self.table.as_mut() {
            table.row_bg0 = Some(color);
        }
    }

    /// `ImGui::TableGetHoveredColumn`: the column under the mouse, `-1` when the table is not
    /// hovered and the column count in the unused space after the last column.
    pub fn table_get_hovered_column(&self) -> i32 {
        self.table
            .as_ref()
            .map(|table| table.hovered_column_body)
            .unwrap_or(-1)
    }

    /// `ImGui::TableGetHoveredRow`: the row under the mouse, one frame behind.
    pub fn table_get_hovered_row(&self) -> i32 {
        self.table
            .as_ref()
            .map(|table| table.hovered_row_last)
            .unwrap_or(-1)
    }

    /// `ImGui::TableGetColumnFlags`: the flags the column ended up with, after the table policy
    /// filled in the width kind, the resize and indentation bits.
    pub fn table_get_column_flags(&self, column: usize) -> u32 {
        self.table
            .as_ref()
            .and_then(|table| table.columns.get(column))
            .map(|c| c.flags)
            .unwrap_or(0)
    }

    /// `ImGui::TableGetColumnName`: the label a column was declared with. It is empty before the
    /// declaration, and `None` once the table is closed (the reference returns NULL there).
    pub fn table_get_column_name(&self, column: usize) -> Option<&str> {
        let table = self.table.as_ref()?;
        if !table.layout_locked && column >= table.decl_columns {
            return Some("");
        }
        table.columns.get(column).map(|c| c.name.as_str())
    }

    /// `ImGui::EndTable`: closes the row, finishes the table's height and lays the whole table out
    /// as one item, which leaves the cursor below it.
    pub fn end_table(&mut self) {
        let Some(mut table) = self.table.take() else {
            return;
        };
        self.table_lock_layout(&mut table);
        if table.is_inside_row {
            self.table_end_row(&mut table);
        }
        // `ImCeil(RowPosY2)`: a plain table is as tall as the bottom of its last row. A scrolling one
        // keeps the size it was asked for; its rows grow the inner child's content instead.
        let inner_content_max_y = table.row_pos_y2.ceil();
        if self.table_child_size.is_none() {
            table.outer_rect.1.y = table.outer_rect.1.y.max(inner_content_max_y);
        }
        // `TableDrawBorders`: the outer rectangle first, then one vertical line at the right edge of
        // every column but the last, from the top of the content to the bottom of the table. All of
        // it goes to the background channel.
        let border_outer_h = if table.flags & crate::tables::table_flags::BORDERS_OUTER_H != 0 {
            1.0
        } else {
            0.0
        };
        let border_outer_v = if table.flags & crate::tables::table_flags::BORDERS_OUTER_V != 0 {
            1.0
        } else {
            0.0
        };
        let borders_outer_h = table.flags & crate::tables::table_flags::BORDERS_OUTER_H != 0;
        let borders_outer_v = table.flags & crate::tables::table_flags::BORDERS_OUTER_V != 0;
        let draw_start = self.draw_list.idx_buffer.len();
        if table.flags & crate::tables::table_flags::BORDERS_INNER_V != 0
            || borders_outer_h
            || borders_outer_v
        {
            let outer_min = table.outer_rect.0;
            let outer_max = table.outer_rect.1;
            let strong = self.style.table_border_strong;
            if borders_outer_h && borders_outer_v {
                self.draw_list
                    .add_rect_stroke(outer_min, outer_max, strong, 1.0);
            } else if borders_outer_v {
                self.draw_list
                    .add_line_v(outer_min.x, outer_min.y, outer_max.y, strong, 1.0);
                self.draw_list.add_line_v(
                    outer_max.x - border_outer_v,
                    outer_min.y,
                    outer_max.y,
                    strong,
                    1.0,
                );
            } else if borders_outer_h {
                self.draw_list
                    .add_line_h(outer_min.x, outer_max.x, outer_min.y, strong, 1.0);
                self.draw_list.add_line_h(
                    outer_min.x,
                    outer_max.x,
                    outer_max.y - border_outer_h,
                    strong,
                    1.0,
                );
            }
        }
        if table.flags & crate::tables::table_flags::BORDERS_INNER_V != 0 {
            let draw_y1 = table.outer_rect.0.y.max(table.work_rect.0.y) + border_outer_h;
            let draw_y2 = table.outer_rect.1.y - border_outer_h;
            // The reference walks the columns in display order, which is the declared order unless a
            // drag reordered them.
            for column_n in table.display_order_to_index.clone() {
                let Some(column) = table.columns.get(column_n as usize) else {
                    continue;
                };
                let mut draw_x2 = column.max_x;
                if draw_x2 >= table.inner_clip.1.x && draw_x2 - 1.0 <= table.inner_clip.1.x {
                    draw_x2 = table.inner_clip.1.x;
                }
                if draw_x2 >= table.inner_clip.1.x {
                    continue;
                }
                // The right-most column's border belongs to the outer rectangle; with the default
                // sizing policy the reference skips it here.
                if column.next_enabled_column == -1 {
                    continue;
                }
                if draw_x2 <= column.clip[0] || draw_y2 <= draw_y1 {
                    continue;
                }
                self.draw_list.add_line_v(
                    draw_x2,
                    draw_y1,
                    draw_y2,
                    self.style.table_border_light,
                    1.0,
                );
            }
        }
        if self.draw_list.idx_buffer.len() > draw_start {
            let tail = self.draw_list.idx_buffer.split_off(draw_start);
            if let Some(cmd) = self.draw_list.cmd_buffer.last_mut() {
                cmd.elem_count -= tail.len() as u32;
            }
            self.table_bg_indices.extend_from_slice(&tail);
        }
        self.draw_list.pop_clip_rect();
        // `TableMergeDrawChannels` + `ImDrawListSplitter::Merge`: the columns whose content stayed
        // inside their clip share one command carrying the window's clip, with the background
        // first; a column whose content overflowed keeps a command of its own with the column's
        // clip, and the merge leaves a trailing command behind when the last one closed.
        let window_clip = self.draw_list.current_clip_rect();
        let background = core::mem::take(&mut self.table_bg_indices);
        let channels = core::mem::take(&mut self.table_indices);
        let fits: Vec<bool> = (0..table.columns.len())
            .map(|n| table.column_content_fits(n))
            .collect();
        // `Bg0ClipRectForDrawCmd` is the **host** window's clip (`TableEndRow` overwrites the command
        // clip with it), which only differs from the table's own clip when the table scrolls inside
        // an inner window. The splitter's merge then joins channels that share a clip, so for a plain
        // table the background and the fitting columns stay in one command.
        let bg_clip = self.table_outer_clip.unwrap_or(window_clip);
        let mut known_clip: Option<[f32; 4]> = None;
        // The command opened when the table began holds no geometry of its own (the cells moved
        // their indices to the channels), so the merge drops it before rebuilding the list.
        self.draw_list.pop_all_unused_commands();
        if !background.is_empty() {
            self.draw_list.start_command(bg_clip);
            self.draw_list.idx_buffer.extend_from_slice(&background);
            if let Some(cmd) = self.draw_list.cmd_buffer.last_mut() {
                cmd.elem_count += background.len() as u32;
            }
            known_clip = Some(bg_clip);
        }
        // `TableMergeDrawChannels` walks the columns **by index** to build the groups and the merge
        // copies the channels in channel-number order, which is the index order (the display order
        // only decides where the geometry was placed, not how the channels are merged).
        let mut shared: Vec<crate::draw_list::ImDrawIdx> = Vec::new();
        for (n, channel) in channels.iter().enumerate() {
            if fits[n] {
                shared.extend_from_slice(channel);
            }
        }
        if !shared.is_empty() {
            if known_clip == Some(window_clip) {
                // The splitter's merge joins the channels that share a clip into one command.
                if let Some(cmd) = self.draw_list.cmd_buffer.last_mut() {
                    cmd.elem_count += shared.len() as u32;
                }
                self.draw_list.idx_buffer.extend_from_slice(&shared);
            } else {
                self.draw_list.start_command(window_clip);
                self.draw_list.idx_buffer.extend_from_slice(&shared);
                if let Some(cmd) = self.draw_list.cmd_buffer.last_mut() {
                    cmd.elem_count += shared.len() as u32;
                }
            }
        }
        for (n, channel) in channels.iter().enumerate() {
            if fits[n] || channel.is_empty() {
                continue;
            }
            let clip = table.columns[n].clip;
            self.draw_list.start_command(clip);
            self.draw_list.idx_buffer.extend_from_slice(channel);
            if let Some(cmd) = self.draw_list.cmd_buffer.last_mut() {
                cmd.elem_count += channel.len() as u32;
            }
        }
        let needs_trailing = self
            .draw_list
            .cmd_buffer
            .last()
            .map(|cmd| cmd.elem_count != 0 && cmd.clip_rect != window_clip)
            .unwrap_or(true);
        if needs_trailing {
            self.draw_list.start_command(window_clip);
        }
        self.pop_id();
        // The content extents are what sizes the columns on the next frame, so they outlive the
        // table instance; the reference keeps them in the `ImGuiTable` it reuses.
        if let Some(id) = self.table_id.take() {
            let height = table.outer_rect.1.y - table.outer_rect.0.y;
            self.table_last_outer_height.insert(id.clone(), height);
            let next_row = table.hovered_row_next;
            self.table_hovered_row_next.insert(id.clone(), next_row);
            let extents = table
                .columns
                .iter()
                .map(|column| column.content_width)
                .collect();
            self.table_content_max_x.insert(id.clone(), extents);
            // `TableSaveSettings` for the sort bits: the orders, the directions and the dirty flags the
            // next frame starts from, kept by table identifier like the content extents.
            self.table_sort_state.insert(id.clone(), table.sort_state());
            // The display order and the reorder request a drag queued for the next `BeginTable`.
            self.table_column_order.insert(
                id,
                (
                    table.display_orders.clone(),
                    table.reorder_column,
                    table.reorder_column_dst_order,
                ),
            );
        }
        // Back at the table's top-left and at the host's line start, then laid out as one item.
        if self.table_child_size.is_some() {
            // `EndChild`: the inner window closes and the host advances by the child's size. The host's
            // cursor and line start were never moved: the rows drew into the child's window.
            self.table_host.take();
            self.end_table_child();
            self.table_outer_clip = None;
            self.table = None;
            return;
        }
        if let Some(window) = self.windows.last_mut() {
            window.cursor = table.outer_rect.0;
            // `EndTable` pops the clip, so the items after the table see the window's again.
            window.clip_rect = None;
            if let Some(host) = self.table_host.take() {
                window.line_start_x = host.line_start_x;
            }
        }
        let size = Vec2 {
            x: table.outer_rect.1.x - table.outer_rect.0.x,
            y: table.outer_rect.1.y - table.outer_rect.0.y,
        };
        self.dummy(size);
        self.table_outer_clip = None;
        self.table = None;
    }

    /// `ImGui::TableGetRowIndex`, and 0 when no table is open.
    pub fn table_get_row_index(&self) -> i32 {
        self.table.as_ref().map(|t| t.current_row).unwrap_or(0)
    }

    /// `ImGui::TableGetColumnIndex`, and 0 when no table is open.
    pub fn table_get_column_index(&self) -> i32 {
        self.table.as_ref().map(|t| t.current_column).unwrap_or(0)
    }

    /// The columns are resolved on the first row or cell, as `TableNextRow` does in the reference.
    fn table_lock_layout(&mut self, table: &mut crate::tables::Table) {
        if table.layout_locked {
            return;
        }
        let host_clip = self.draw_list.current_clip_rect();
        table.update_layout(
            self.style.cell_padding,
            host_clip,
            self.style.frame_padding.x,
        );
        // The hover detection of `TableUpdateLayout`: the row is one frame behind, and the column is
        // the one whose clip holds the mouse, or the column count after the last one.
        table.hovered_row_last = table.hovered_row_next;
        table.hovered_row_next = -1;
        table.hovered_column_body = -1;
        let last_height = self
            .table_id
            .as_ref()
            .and_then(|id| self.table_last_outer_height.get(id))
            .copied()
            .unwrap_or(0.0);
        let hit_max = Vec2 {
            x: table.outer_rect.1.x,
            y: table.outer_rect.1.y.max(table.outer_rect.0.y + last_height),
        };
        for column in table.columns.iter_mut() {
            column.flags &= !crate::tables::column_flags::IS_HOVERED;
        }
        if self.hovered_in(table.outer_rect.0, hit_max) {
            if let Some(mouse) = self.io.mouse_pos {
                for (n, column) in table.columns.iter_mut().enumerate() {
                    if mouse.x >= column.clip[0] && mouse.x < column.clip[2] {
                        column.flags |= crate::tables::column_flags::IS_HOVERED;
                        table.hovered_column_body = n as i32;
                    }
                }
                if table.hovered_column_body == -1 {
                    let unused_x1 = table.columns.last().map(|c| c.clip[2]).unwrap_or(0.0);
                    if mouse.x >= unused_x1 {
                        table.hovered_column_body = table.columns.len() as i32;
                    }
                }
            }
        }
    }

    /// `TableEndRow`: closes the open cell and leaves the cursor at the row's bottom.
    fn table_end_row(&mut self, table: &mut crate::tables::Table) {
        // `TableGetHoveredRow` data: the first row whose span holds the mouse while a column is
        // hovered.
        if table.hovered_column_body != -1 {
            if let Some(mouse) = self.io.mouse_pos {
                if mouse.y >= table.row_pos_y1
                    && mouse.y < table.row_pos_y2
                    && table.hovered_row_next < 0
                {
                    table.hovered_row_next = table.current_row;
                }
            }
        }
        if table.current_column != -1 {
            self.table_end_cell(table);
            table.current_column = -1;
        }
        if let Some(window) = self.windows.last_mut() {
            window.cursor.y = table.row_pos_y2;
        }
        // `TableEndRow` paints the row background, soft-clipped to `BgClipRect`, into the background
        // channel so it merges before the columns.
        if let Some(color) = table.row_bg0 {
            let clip = table.bg_clip;
            let min = Vec2 {
                x: table.work_rect.0.x.max(clip.0.x),
                y: table.row_pos_y1.max(clip.0.y),
            };
            let max = Vec2 {
                x: table.work_rect.1.x.min(clip.1.x),
                y: table.row_pos_y2.min(clip.1.y),
            };
            // A fully transparent row background (`TableRowBg` is 0,0,0,0) draws nothing at all.
            if color != 0 && min.y < max.y {
                let start = self.draw_list.idx_buffer.len();
                self.draw_list.add_rect_filled(min, max, color);
                let tail = self.draw_list.idx_buffer.split_off(start);
                if let Some(cmd) = self.draw_list.cmd_buffer.last_mut() {
                    cmd.elem_count -= tail.len() as u32;
                }
                self.table_bg_indices.extend_from_slice(&tail);
            }
        }
        // `TableEndRow` paints the cell backgrounds in submission order, in the same channel.
        for (column, color) in table.row_cells.clone() {
            let rect = table.cell_bg_rect(column);
            if rect.0.y < rect.1.y {
                let start = self.draw_list.idx_buffer.len();
                self.draw_list.add_rect_filled(rect.0, rect.1, color);
                let tail = self.draw_list.idx_buffer.split_off(start);
                if let Some(cmd) = self.draw_list.cmd_buffer.last_mut() {
                    cmd.elem_count -= tail.len() as u32;
                }
                self.table_bg_indices.extend_from_slice(&tail);
            }
        }
        // `TableEndRow` draws the border above a row once it has another row before it, in the
        // strong colour when the previous row was a header row.
        if table.current_row > 0 && table.flags & crate::tables::table_flags::BORDERS_INNER_H != 0 {
            let y = table.row_pos_y1;
            if y >= table.bg_clip.0.y && y < table.bg_clip.1.y {
                let colour = if table.last_row_flags & crate::tables::table_flags::ROW_HEADERS != 0
                {
                    self.style.table_border_strong
                } else {
                    self.style.table_border_light
                };
                let start = self.draw_list.idx_buffer.len();
                self.draw_list
                    .add_line_h(table.border_x1, table.border_x2, y, colour, 1.0);
                let tail = self.draw_list.idx_buffer.split_off(start);
                if let Some(cmd) = self.draw_list.cmd_buffer.last_mut() {
                    cmd.elem_count -= tail.len() as u32;
                }
                self.table_bg_indices.extend_from_slice(&tail);
            }
        }
        table.is_inside_row = false;
    }

    /// `TableBeginCell`: the content of a cell starts at its column's work edge, one row padding
    /// down, and the line start moves there so the items of the cell stay inside the column.
    fn table_begin_cell(&mut self, table: &mut crate::tables::Table, column: usize) {
        table.current_column = column as i32;
        let start = table.cell_min(column);
        let clip = table.columns[column].clip;
        self.cell_index_start = self.draw_list.idx_buffer.len();
        if let Some(window) = self.windows.last_mut() {
            window.cursor = start;
            window.line_start_x = start.x;
            window.line_height = 0.0;
            window.cursor_max.x = start.x;
            // `SetWindowClipRectBeforeSetChannel`: the items of a cell are clipped to its column.
            window.clip_rect = Some(clip);
        }
    }

    /// `TableEndCell`: the furthest content of the cell grows the row by one cell padding, and the
    /// indices the cell drew move to their column's draw channel.
    fn table_end_cell(&mut self, table: &mut crate::tables::Table) {
        let (max_x, max_y) = {
            let cursor_max = self.window().cursor_max;
            (cursor_max.x, cursor_max.y)
        };
        table.end_cell(max_y);
        let column = table.current_column.max(0) as usize;
        if let Some(col) = table.columns.get_mut(column) {
            col.content_max_x = col.content_max_x.max(max_x);
            col.content_width = col.content_max_x - col.work_min_x;
        }
        let column = table.current_column.max(0) as usize;
        let tail = self.draw_list.idx_buffer.split_off(self.cell_index_start);
        if let Some(cmd) = self.draw_list.cmd_buffer.last_mut() {
            cmd.elem_count -= tail.len() as u32;
        }
        if let Some(channel) = self.table_indices.get_mut(column) {
            channel.extend_from_slice(&tail);
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

    /// `ImGuiItemFlags_Disabled`: `ItemHoverable` returns false, so the item neither hovers, nor
    /// becomes active, nor reports a press.
    fn is_disabled(&self) -> bool {
        self.disabled_depth > 0
    }

    /// The hover/active/press logic shared by framed widgets (`ButtonBehavior`).
    fn interact(&mut self, id: ImGuiID, size: Vec2) -> bool {
        let min = self.window().cursor;
        let max = Vec2 {
            x: min.x + size.x,
            y: min.y + size.y,
        };
        let (hovered, active, pressed) = self.behaviour_at(id, min, max, false);
        self.dummy(size);
        self.last_item_hovered = hovered;
        self.last_item_active = active;
        pressed
    }

    /// ButtonBehavior over a rectangle: hover, press on release, and the active state.
    fn behaviour_at(
        &mut self,
        id: ImGuiID,
        min: Vec2,
        max: Vec2,
        press_on_click: bool,
    ) -> (bool, bool, bool) {
        if self.is_disabled() {
            return (false, false, false);
        }
        let (window_pos, window_size) = {
            let window = self.window();
            (window.pos, window.size)
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
        // `ButtonBehavior`: an item only hovers while its own window is the one under the mouse, which
        // is what keeps the items of a popup from reacting through the window that hosts it.
        // `update_resize` runs before the window is on the stack, so this must not assume one.
        let own_window = match self.windows.last() {
            Some(window) => self.hovered_window.as_deref() == Some(window.name.as_str()),
            None => false,
        };
        let hovered = own_window
            && over_window
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
        // On release the reference clears the active id before returning, so
        // IsItemActive() queried afterwards is already false on that frame.
        let was_active = self.active_id == Some(id);
        let mut pressed = false;
        // ImGuiButtonFlags_PressedOnClick: the press itself activates, which is what the arrow of a
        // tree node does so that opening reacts on the way down.
        if press_on_click && hovered && self.io.mouse_clicked[0] {
            pressed = true;
        }
        if was_active && self.io.mouse_released[0] {
            // A press-to-activate item was already activated on the way down, so releasing it must
            // not press it a second time.
            pressed = pressed || (!press_on_click && hovered);
            self.active_id = None;
        }
        // held is the id being active *and* the button still down, which is what the colours key on.
        (
            hovered,
            self.active_id == Some(id) && self.io.mouse_down[0],
            pressed,
        )
    }
    /// `ImGui::Button`: a framed label. Returns true on the frame the mouse is released inside.
    pub fn button(&mut self, label: &str) -> bool {
        // `SkipItems`: a collapsed window lays nothing out, so an item submitted into it draws
        // nothing and reports no press.
        if self
            .windows
            .last()
            .map(|window| window.collapsed)
            .unwrap_or(false)
        {
            return false;
        }
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

    /// `ImGui::CollapsingHeader`: a framed, full-width header with a collapsing arrow. Returns true
    /// while it is open, and the caller decides what to draw under it.
    ///
    /// This is `TreeNodeBehavior` with the `CollapsingHeader` flag set, which adds `Framed`,
    /// `SpanAvailWidth` and `NoTreePushOnOpen`. Left out is everything that needs machinery this port
    /// does not have yet: keyboard navigation, multi-selection, drag and drop, and the tree lines.
    pub fn collapsing_header(&mut self, label: &str) -> bool {
        let id = self.get_id(label);
        let pad = self.style.frame_padding;
        let text = self.font.calc_text_size(label, self.font.size);
        // The arrow takes `FontSize` plus a frame padding on each side.
        let icon_width = self.font.size + pad.x * 2.0;
        let text_offset_x = icon_width + pad.x;
        let text_offset_y = pad.y;
        let frame_height = text.y + pad.y * 2.0;
        // A framed header reaches a little outside the current limits.
        let outer_extend = (self.style.window_padding.x * 0.5).trunc();

        let (cursor, work_max_x) = {
            let window = self.window();
            (
                window.cursor,
                window.inner_rect.1.x - self.style.window_padding.x,
            )
        };
        let frame_min = Vec2 {
            x: cursor.x - outer_extend,
            y: cursor.y,
        };
        let frame_max = Vec2 {
            x: work_max_x + outer_extend,
            y: cursor.y + frame_height,
        };
        let text_pos = Vec2 {
            x: cursor.x + text_offset_x,
            y: cursor.y + text_offset_y,
        };

        // `ItemSize(ImVec2(text_width, frame_height), padding.y)`: the item is as wide as the arrow
        // plus the label, even though the header itself spans the window.
        let item = Vec2 {
            x: icon_width + text.x,
            y: frame_height,
        };
        // The arrow area activates on press, the rest of the header on release.
        let over_arrow = self
            .io
            .mouse_pos
            .map(|m| m.x >= cursor.x && m.x < cursor.x + icon_width)
            .unwrap_or(false);
        let pressed = self.interact_rect(id, frame_min, frame_max, item, over_arrow);
        let mut open = self.item_open(id);
        if pressed {
            open = !open;
            self.set_item_open(id, open);
        }

        // `RenderFrame` with the header colours, then the arrow, then the label.
        let bg_col = if self.last_item_active {
            self.style.color_u32(crate::style::Col::HeaderActive)
        } else if self.last_item_hovered {
            self.style.color_u32(crate::style::Col::HeaderHovered)
        } else {
            self.style.color_u32(crate::style::Col::Header)
        };
        self.draw_list.prim_rect(frame_min, frame_max, bg_col);
        if self.style.frame_border_size > 0.0 {
            self.draw_list.add_aligned_rect_border(
                frame_min,
                frame_max,
                self.style.color_u32(crate::style::Col::Border),
                self.style.frame_border_size,
            );
        }
        let text_col = self.style.color_u32(crate::style::Col::Text);
        // `RenderArrow(... ImVec2(text_pos.x - text_offset_x + padding.x, text_pos.y) ...)`: down
        // while it is open, right while it is closed.
        let (cos_a, sin_a) = if open {
            (ARROW_DOWN_COS, ARROW_DOWN_SIN)
        } else {
            (ARROW_RIGHT_COS, ARROW_RIGHT_SIN)
        };
        self.draw_list.add_arrow(
            Vec2 {
                x: text_pos.x - text_offset_x + pad.x,
                y: text_pos.y,
            },
            text_col,
            cos_a,
            sin_a,
            1.0,
            self.font.size,
            self.style.symbol_arrow_size_factor,
            self.style.symbol_arrow_thickness_factor,
        );
        let font = self.font.clone();
        self.draw_list
            .add_text(&font, font.size, text_pos, text_col, label);
        open
    }

    /// `TreeNodeUpdateNextOpen` with no next-item data and no default-open flag: closed until the
    /// user opens it.
    fn item_open(&self, id: ImGuiID) -> bool {
        self.item_states
            .iter()
            .find(|(known, _)| *known == id)
            .map(|(_, open)| *open)
            .unwrap_or(false)
    }

    fn set_item_open(&mut self, id: ImGuiID, open: bool) {
        if let Some(entry) = self.item_states.iter_mut().find(|(known, _)| *known == id) {
            entry.1 = open;
            return;
        }
        self.item_states.push((id, open));
    }

    /// `ButtonBehavior` over an explicit rectangle, with `ItemSize` advancing the cursor by `item`.
    fn interact_rect(
        &mut self,
        id: ImGuiID,
        min: Vec2,
        max: Vec2,
        item: Vec2,
        press_on_click: bool,
    ) -> bool {
        let (hovered, held, pressed) = self.behaviour_at(id, min, max, press_on_click);
        self.dummy(item);
        self.last_item_hovered = hovered;
        self.last_item_active = held;
        pressed
    }

    /// `ImGui::TreeNode`: an unframed tree node. Returns true while it is open, and `tree_pop` closes
    /// it.
    ///
    /// This is `TreeNodeBehavior` with no flags: the arrow takes a font size plus a frame padding on
    /// each side, the row only reacts where the arrow and the text are, but the hover highlight spans
    /// the work area, and the arrow is drawn smaller than a framed header's, at 0.70.
    pub fn tree_node(&mut self, label: &str) -> bool {
        let id = self.get_id(label);
        let pad = self.style.frame_padding;
        let text = self.font.calc_text_size(label, self.font.size);
        let icon_width = self.font.size + pad.x * 2.0;
        // Not framed, so the label sits right after the arrow and the row is one line tall.
        let text_offset_x = icon_width;
        let frame_height = text.y;

        let (cursor, work_max_x) = {
            let window = self.window();
            (
                window.cursor,
                window.inner_rect.1.x - self.style.window_padding.x,
            )
        };
        let frame_min = cursor;
        let frame_max = Vec2 {
            x: work_max_x,
            y: cursor.y + frame_height,
        };
        let text_pos = Vec2 {
            x: cursor.x + text_offset_x,
            y: cursor.y,
        };
        // Without a span flag the row only reacts where the arrow and the text are.
        let text_width = icon_width + text.x;
        let interact_max_x = frame_min.x + text_width + self.style.item_spacing.x * 2.0;

        let item = Vec2 {
            x: text_width,
            y: frame_height,
        };
        let over_arrow = self
            .io
            .mouse_pos
            .map(|m| m.x >= cursor.x && m.x < cursor.x + icon_width)
            .unwrap_or(false);
        let pressed = self.interact_rect_spanning(
            id,
            frame_min,
            Vec2 {
                x: interact_max_x,
                y: frame_max.y,
            },
            item,
            over_arrow,
        );
        let mut open = self.item_open(id);
        if pressed {
            open = !open;
            self.set_item_open(id, open);
        }

        let text_col = self.style.color_u32(crate::style::Col::Text);
        if self.last_item_hovered || self.last_item_active {
            let bg = if self.last_item_active && self.last_item_hovered {
                crate::style::Col::HeaderActive
            } else if self.last_item_hovered {
                crate::style::Col::HeaderHovered
            } else {
                crate::style::Col::Header
            };
            let colour = self.style.color_u32(bg);
            self.draw_list.prim_rect(frame_min, frame_max, colour);
        }
        // `RenderArrow(..., scale 0.70)` sits a little right and down of the text corner.
        let (cos_a, sin_a) = if open {
            (ARROW_DOWN_COS, ARROW_DOWN_SIN)
        } else {
            (ARROW_RIGHT_COS, ARROW_RIGHT_SIN)
        };
        let lift = self.font.size * 0.15;
        self.draw_list.add_arrow(
            Vec2 {
                x: text_pos.x + lift - text_offset_x + pad.x,
                y: text_pos.y + lift,
            },
            text_col,
            cos_a,
            sin_a,
            0.70,
            self.font.size,
            self.style.symbol_arrow_size_factor,
            self.style.symbol_arrow_thickness_factor,
        );
        let font = self.font.clone();
        self.draw_list
            .add_text(&font, font.size, text_pos, text_col, label);

        if open {
            // `TreePushOverrideID`: indent the children and give them ids under this node.
            self.indent(None);
            self.push_override_id(id);
        }
        open
    }

    /// `ImGui::TreePop`: undo what an open `tree_node` did.
    pub fn tree_pop(&mut self) {
        self.pop_id();
        self.unindent(None);
    }

    /// `Indent`: move the layout's left edge right and put the cursor there.
    pub fn indent(&mut self, width: Option<f32>) {
        let width = width.unwrap_or(self.style.indent_spacing);
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        window.line_start_x += width;
        window.cursor.x = window.line_start_x;
    }

    /// `Unindent`.
    pub fn unindent(&mut self, width: Option<f32>) {
        let width = width.unwrap_or(self.style.indent_spacing);
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        window.line_start_x -= width;
        window.cursor.x = window.line_start_x;
    }

    /// `ImGui::InvisibleButton`: an item of the given size that draws nothing but hovers and reports
    /// a press, which is what a custom widget builds on.
    pub fn invisible_button(&mut self, id_str: &str, size: Vec2) -> bool {
        if size.x == 0.0 || size.y == 0.0 {
            return false;
        }
        let id = self.get_id(id_str);
        self.interact(id, size)
    }

    /// `ButtonBehavior` over a rectangle, with the item's own width for `ItemSize`: the row a tree
    /// node reacts to is narrower than the highlight it paints.
    fn interact_rect_spanning(
        &mut self,
        id: ImGuiID,
        min: Vec2,
        max: Vec2,
        item: Vec2,
        press_on_click: bool,
    ) -> bool {
        let (hovered, held, pressed) = self.behaviour_at(id, min, max, press_on_click);
        self.dummy(item);
        self.last_item_hovered = hovered;
        self.last_item_active = held;
        pressed
    }

    /// `window->WorkRect.Max.x`: the right edge a widget may fill, which is the interior's left edge
    /// plus the inner width minus the padding on both sides.
    fn work_rect_max_x(&self) -> f32 {
        let window = self.window();
        let pad = self.style.window_padding.x;
        let border = self.style.window_border_size;
        let min_x = (window.inner_rect.0.x - window.scroll.x + pad.max(border)).trunc();
        let inner_width = window.inner_rect.1.x - window.inner_rect.0.x;
        min_x + (inner_width - pad * 2.0).max(0.0)
    }

    /// `ImGui::Selectable`: a row that fills the available width and highlights when hovered or
    /// selected. Returns true on the frame it is clicked.
    ///
    /// The row advances by the label height with no padding of its own, while the clickable box is
    /// expanded by half the item spacing on each side so that consecutive rows leave no gap the
    /// mouse can fall into.
    pub fn selectable(&mut self, label: &str, selected: bool) -> bool {
        self.selectable_box(label, selected, Vec2::default(), false, false, true)
    }

    /// `ImGui::Text` for an unformatted string: an item as wide as the text, no interaction.
    pub fn label(&mut self, text: &str) {
        let size = self.font.calc_text_size(text, self.font.size);
        self.dummy(size);
    }

    /// `ImGui::LabelText`: a value aligned to the item width, then its label on the same line after
    /// `ItemInnerSpacing.x`, both offset by `FramePadding` from the item's top.
    pub fn label_text(&mut self, label: &str, value: &str) {
        if self
            .windows
            .last()
            .map(|window| window.collapsed)
            .unwrap_or(false)
        {
            return;
        }
        let width = self.calc_item_width();
        let frame_padding = self.style.frame_padding;
        let inner_spacing = self.style.item_inner_spacing;
        let font = self.font.clone();
        let value_size = font.calc_text_size(value, font.size);
        let label_shown = rendered_text(label);
        let label_size = font.calc_text_size(label_shown, font.size);
        let pos = self.window().cursor;
        let value_bb = (
            pos,
            Vec2 {
                x: pos.x + width,
                y: pos.y + value_size.y + frame_padding.y * 2.0,
            },
        );
        let total_size = Vec2 {
            x: width
                + if label_size.x > 0.0 {
                    inner_spacing.x + label_size.x
                } else {
                    0.0
                },
            y: value_size.y.max(label_size.y) + frame_padding.y * 2.0,
        };
        // `ItemSize(total_bb, FramePadding.y)` then `ItemAdd(total_bb, 0)`.
        self.dummy(total_size);
        if let Some(window) = self.windows.last_mut() {
            window.last_max = Vec2 {
                x: pos.x + total_size.x,
                y: pos.y + total_size.y,
            };
            window.has_item = true;
        }
        let colour = self.style.color_u32(crate::style::Col::Text);
        let clip = self.window().clip_rect;
        // `RenderTextClipped` with `align (0, 0)` over the value's box.
        self.draw_list.add_text_in(
            &font,
            font.size,
            Vec2 {
                x: pos.x + frame_padding.x,
                y: pos.y + frame_padding.y,
            },
            colour,
            value,
            clip,
            Some((value_bb.0, value_bb.1)),
        );
        if label_size.x > 0.0 {
            self.draw_list.add_text_in(
                &font,
                font.size,
                Vec2 {
                    x: value_bb.1.x + inner_spacing.x,
                    y: pos.y + frame_padding.y,
                },
                colour,
                label_shown,
                clip,
                None,
            );
        }
    }

    /// `ImGui::Text` with the text actually painted, which is what a tooltip shows.
    pub fn text(&mut self, text: &str) {
        let colour = self.style.color_u32(crate::style::Col::Text);
        self.text_with_colour(text, colour);
    }

    /// `ImGui::TextColored`: the text painted with an explicit colour. The reference pushes it as
    /// `ImGuiCol_Text`, so `Style::Alpha` still applies, which is what `Style::color_u32_from` does.
    pub fn text_colored(&mut self, text: &str, colour: [f32; 4]) {
        let packed = self.style.color_u32_from(colour);
        self.text_with_colour(text, packed);
    }

    /// `ImGui::TextDisabled`: the text in `ImGuiCol_TextDisabled`.
    pub fn text_disabled(&mut self, text: &str) {
        let colour = self.style.color_u32(crate::style::Col::TextDisabled);
        self.text_with_colour(text, colour);
    }

    /// The body shared by every text widget: paint at the cursor with a packed colour, then take the
    /// text's size in the layout.
    fn text_with_colour(&mut self, text: &str, colour: u32) {
        let font = self.font.clone();
        let pos = self.window().cursor;
        let clip = self.window().clip_rect;
        let size = font.calc_text_size(text, font.size);
        self.draw_list
            .add_text_in(&font, font.size, pos, colour, text, clip, None);
        self.dummy(size);
    }

    /// `ImGui::Separator`: in a vertical layout, a line of `Style::SeparatorSize` pixels across the
    /// window's work width laid out as an item of its thickness; in a horizontal one (the menu bar) a
    /// vertical line between the items of the current line.
    pub fn separator(&mut self) {
        let thickness = self.style.separator_size.max(1.0);
        let Some(window) = self.windows.last() else {
            return;
        };
        // `SkipItems`: a collapsed window lays nothing out.
        if window.collapsed {
            return;
        }
        // `SeparatorEx` with `ImGuiSeparatorFlags_Vertical`: the line spans the height the current
        // line already reached, and only its width goes into the layout.
        if window.layout_horizontal {
            let bb = (
                window.cursor,
                Vec2 {
                    x: window.cursor.x + thickness,
                    y: window.cursor.y + window.line_height,
                },
            );
            let colour = self.style.color_u32(crate::style::Col::Separator);
            self.draw_list.add_rect_filled(bb.0, bb.1, colour);
            self.dummy(Vec2 {
                x: thickness,
                y: 0.0,
            });
            if let Some(window) = self.windows.last_mut() {
                window.last_min = bb.0;
                window.last_max = bb.1;
                window.has_item = true;
            }
            return;
        }
        let bb = (
            window.cursor,
            Vec2 {
                x: window.content_origin.x + window.content_size.x,
                y: window.cursor.y + thickness,
            },
        );
        let colour = self.style.color_u32(crate::style::Col::Separator);
        self.draw_list.add_rect_filled(bb.0, bb.1, colour);
        // `ItemSize(ImVec2(0, thickness))`: the line does not feed its width to the layout, and the
        // item rect is the line itself.
        self.dummy(Vec2 {
            x: 0.0,
            y: thickness,
        });
        if let Some(window) = self.windows.last_mut() {
            window.last_min = bb.0;
            window.last_max = bb.1;
            window.has_item = true;
        }
    }

    /// `ImGui::SeparatorText`: the label between two lines, as wide as the window's work width and as
    /// tall as the label plus `Style::SeparatorTextPadding`, with the lines at
    /// `Style::SeparatorTextBorderSize` around the label.
    pub fn separator_text(&mut self, label: &str) {
        // `FindRenderedTextEnd`: everything after "##" identifies the item but is not drawn.
        let shown = match label.find("##") {
            Some(at) => &label[..at],
            None => label,
        };
        let font = self.font.clone();
        let label_size = font.calc_text_size(shown, font.size);
        let Some(window) = self.windows.last() else {
            return;
        };
        if window.collapsed {
            return;
        }
        let pos = window.cursor;
        let work_max_x = window.content_origin.x + window.content_size.x;
        let padding = self.style.separator_text_padding;
        let thickness = self.style.separator_text_border_size;
        let align = self.style.separator_text_align;
        let min_size = Vec2 {
            x: label_size.x + padding.x * 2.0,
            y: (label_size.y + padding.y * 2.0).max(thickness),
        };
        let bb_max = Vec2 {
            x: work_max_x,
            y: pos.y + min_size.y,
        };
        // `ImTrunc((bb.GetHeight() - label_size.y) * SeparatorTextAlign.y + 0.999f)`: the label sits
        // half a pixel down so it lands centred inside the item.
        let text_baseline_y = ((bb_max.y - pos.y - label_size.y) * align.y + 0.999).trunc();
        // `ItemSize(min_size, text_baseline_y)` then `ItemAdd(bb, id)`: the item is as wide as the
        // label, and its rectangle spans the whole work width.
        self.dummy(min_size);
        if let Some(window) = self.windows.last_mut() {
            window.last_min = pos;
            window.last_max = bb_max;
            window.has_item = true;
        }
        let separator_x1 = pos.x;
        let separator_x2 = bb_max.x;
        let lines_y = ((pos.y + bb_max.y) * 0.5).trunc() + 1.0;
        let label_avail_w = (separator_x2 - separator_x1 - padding.x * 2.0).max(0.0);
        let label_pos = Vec2 {
            x: pos.x + padding.x + ((label_avail_w - label_size.x).max(0.0) * align.x),
            y: pos.y + text_baseline_y,
        };
        let line_colour = self.style.color_u32(crate::style::Col::Separator);
        let text_colour = self.style.color_u32(crate::style::Col::Text);
        if label_size.x > 0.0 {
            let first_x2 = label_pos.x - self.style.item_spacing.x;
            let second_x1 = label_pos.x + label_size.x + self.style.item_spacing.x;
            if first_x2 > separator_x1 && thickness > 0.0 {
                self.draw_list.add_line_h_center_biased(
                    separator_x1,
                    first_x2,
                    lines_y,
                    line_colour,
                    thickness,
                );
            }
            if separator_x2 > second_x1 && thickness > 0.0 {
                self.draw_list.add_line_h_center_biased(
                    second_x1,
                    separator_x2,
                    lines_y,
                    line_colour,
                    thickness,
                );
            }
            let clip = self.window().clip_rect;
            self.draw_list
                .add_text_in(&font, font.size, label_pos, text_colour, shown, clip, None);
        } else if thickness > 0.0 {
            self.draw_list.add_line_h_center_biased(
                separator_x1,
                separator_x2,
                lines_y,
                line_colour,
                thickness,
            );
        }
    }

    /// `ImGui::BeginMenuBar`: the bar along the top of the window, with its items laid out in a row.
    ///
    /// The bar's background is part of the window's decorations, so this only clips to the bar, moves
    /// the cursor to its start and switches the layout to horizontal.
    pub fn begin_menu_bar(&mut self) {
        let (bar_min, bar_max, pad) = {
            let window = self.window();
            (
                Vec2 {
                    x: window.inner_rect.0.x,
                    y: window.inner_rect.0.y,
                },
                Vec2 {
                    x: window.inner_rect.0.x + window.size_full.x,
                    y: window.inner_rect.0.y + window.menu_bar_height,
                },
                self.style.window_padding,
            )
        };
        // `clip_rect(IM_ROUND(bar.Min + border * 0.5), IM_ROUND(bar.Max - max(rounding, border * 0.5)))`:
        // the bar is clipped inside the window border, with the right edge pulled in by the rounding.
        let border = self.style.window_border_size;
        let rounding = self.style.window_rounding;
        let clip = (
            Vec2 {
                x: (bar_min.x + border * 0.5).round(),
                y: (bar_min.y + border * 0.5).round(),
            },
            Vec2 {
                x: (bar_max.x - rounding.max(border * 0.5)).round(),
                y: bar_max.y.round(),
            },
        );
        self.draw_list.push_clip_rect(clip.0, clip.1, false);
        // `PushID("##MenuBar")`: the entries of the bar get their own identifier scope.
        self.push_id("##MenuBar");
        // `FindBestWindowPosForPopup` places a child menu below its parent's clip rectangle, which
        // while a menu bar is open is the bar itself. This is the edge it measures from.
        self.menu_bar_clip_max_y = Some(clip.1.y);
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        window.layout_horizontal = true;
        window.cursor = Vec2 {
            x: bar_min.x + pad.x,
            y: bar_min.y,
        };
        window.line_start_x = window.cursor.x;
        // `AlignTextToFramePadding`: the bar's line is already as tall as a framed item, so an item
        // that reads the current line height (a vertical Separator) spans the whole bar.
        window.line_height = self.font.size + self.style.frame_padding.y * 2.0;
    }

    /// `ImGui::EndMenuBar`: back to a vertical layout, with the content starting below the bar.
    pub fn end_menu_bar(&mut self) {
        self.pop_id();
        self.draw_list.pop_clip_rect();
        let pad = self.style.window_padding;
        let Some(window) = self.windows.last_mut() else {
            return;
        };
        window.layout_horizontal = false;
        window.cursor = Vec2 {
            x: window.inner_rect.0.x + pad.x - window.scroll.x,
            y: window.inner_rect.0.y + pad.y + window.menu_bar_height - window.scroll.y,
        };
        window.line_start_x = window.cursor.x;
        window.line_height = 0.0;
    }

    /// `window->DC.CurrLineTextBaseOffset`: a menu bar aligns its entries to the frame padding, and a
    /// vertical menu does not offset its rows at all.
    fn line_text_base_offset(&self) -> f32 {
        if self.window().layout_horizontal {
            self.style.frame_padding.y
        } else {
            0.0
        }
    }

    /// `window->ContentRegionRect.Max.x`: the right edge `GetContentRegionAvail` measures from,
    /// which is the interior less one window padding on each side.
    fn content_region_max_x(&self) -> f32 {
        let window = self.window();
        let pad = self.style.window_padding.x;
        let min_x = window.inner_rect.0.x - window.scroll.x + pad;
        min_x + (window.inner_rect.1.x - window.inner_rect.0.x - pad * 2.0).max(0.0)
    }

    /// `ImGui::Selectable`, with the flags the menu code passes: an explicit size, whether the row
    /// spans the available width, whether the press lands on the way down, and whether choosing the
    /// row closes the popup it lives in.
    ///
    /// The layout item and the clickable box are told apart, as the reference does: `ItemSize` runs
    /// first with `size_arg`, and only then is the box widened to the work rectangle.
    fn selectable_box(
        &mut self,
        label: &str,
        selected: bool,
        size_arg: Vec2,
        span_avail_width: bool,
        press_on_click: bool,
        auto_close: bool,
    ) -> bool {
        let id = self.get_id(label);
        let text = self.font.calc_text_size(label, self.font.size);
        let spacing = self.style.item_spacing;
        let max_x = self.work_rect_max_x();
        let base = self.line_text_base_offset();
        // `pos = window->DC.CursorPos; pos.y += window->DC.CurrLineTextBaseOffset`: the text baseline
        // moves the box down, while `ItemSize` below still advances from the cursor itself.
        let cursor = self.window().cursor;
        let pos = Vec2 {
            x: cursor.x,
            y: cursor.y + base,
        };
        // `size(size_arg.x != 0 ? size_arg.x : label_size.x, size_arg.y != 0 ? ... : label_size.y)`.
        let mut size = Vec2 {
            x: if size_arg.x != 0.0 {
                size_arg.x
            } else {
                text.x
            },
            y: if size_arg.y != 0.0 {
                size_arg.y
            } else {
                text.y
            },
        };
        // `ItemSize` runs with this width, before the box is widened below: that is what keeps a
        // full-width row from reporting the whole content region as its own width.
        let item = size;
        // `ImGuiSelectableFlags_SpanAvailWidth`, or no size given: the row runs to the work edge.
        if size_arg.x == 0.0 || span_avail_width {
            size.x = text.x.max(max_x - pos.x);
        }
        // Selectables are meant to be tightly packed, so the box covers the spacing between them.
        let spacing_l = (spacing.x * 0.5).trunc();
        let spacing_u = (spacing.y * 0.5).trunc();
        let click_min = Vec2 {
            x: pos.x - spacing_l,
            y: pos.y - spacing_u,
        };
        let click_max = Vec2 {
            x: pos.x + size.x + (spacing.x - spacing_l),
            y: pos.y + size.y + (spacing.y - spacing_u),
        };
        let pressed = self.interact_rect_spanning(id, click_min, click_max, item, press_on_click);
        let highlighted = self.last_item_hovered;
        let held = self.last_item_active;
        if highlighted || selected {
            let col = if held && highlighted {
                crate::style::Col::HeaderActive
            } else if highlighted {
                crate::style::Col::HeaderHovered
            } else {
                crate::style::Col::Header
            };
            let colour = self.style.color_u32(col);
            let rounding = self.style.selectable_rounding;
            if rounding > 0.0 {
                self.draw_list
                    .add_rect_filled_aa_rounded(click_min, click_max, rounding, colour);
            } else {
                self.draw_list.prim_rect(click_min, click_max, colour);
            }
        }
        let font = self.font.clone();
        let text_col = self.style.color_u32(crate::style::Col::Text);
        self.draw_list
            .add_text(&font, font.size, pos, text_col, label);
        // `CloseCurrentPopup` from `Selectable`: choosing a row of a popup closes it, and that frame
        // still draws the popup, so the close lands on the next one.
        if pressed && auto_close {
            self.close_popup_from_selectable();
        }
        pressed
    }

    /// The tail of `Selectable`: a press inside a popup closes it and hands the focus back to the
    /// window the popup belonged to.
    fn close_popup_from_selectable(&mut self) {
        let popup = self
            .windows
            .last()
            .map(|window| window.flags.contains(WindowFlags::POPUP))
            .unwrap_or(false);
        if !popup {
            return;
        }
        self.popup_pending_close = true;
        // `ClosePopupToLevel` focuses the window the popup belonged to, and `FocusWindow` moves it
        // to the end of the display order, so the host is drawn on top that frame.
        if let Some(host) = self.popup_host.clone() {
            if let Some(index) = self.display_order.iter().position(|name| *name == host) {
                let host = self.display_order.remove(index);
                self.display_order.push(host);
            }
        }
    }

    /// `ImGui::MenuItem` inside a menu bar: an unframed entry that reports the click.
    ///
    /// The entry is laid out the way `MenuItemEx` does in a horizontal layout: the cursor moves half
    /// an item spacing right, the item spacing doubles while the box is measured (which is what
    /// makes consecutive entries butt up against each other), and the box is as tall as the label
    /// alone, not as the line.
    pub fn menu_item(&mut self, label: &str) -> bool {
        self.menu_item_ex(label, None)
    }

    /// `ImGui::MenuItem`: an entry of a menu bar, or a row of a vertical menu once [`Context::begin_menu`]
    /// has opened one. `shortcut` is drawn in the third column of a vertical menu and ignored in a bar,
    /// as the reference does.
    pub fn menu_item_ex(&mut self, label: &str, shortcut: Option<&str>) -> bool {
        if self.window().layout_horizontal {
            self.menu_item_bar(label)
        } else {
            self.menu_item_column(label, shortcut)
        }
    }

    /// `MenuItemEx` in a horizontal menu bar.
    fn menu_item_bar(&mut self, label: &str) -> bool {
        let shown = rendered_text(label);
        let label_size = self.font.calc_text_size(shown, self.font.size);
        let spacing = self.style.item_spacing;
        let base = self.line_text_base_offset();
        let pos = self.window().cursor;
        self.window_mut().cursor.x += (spacing.x * 0.5).trunc();
        self.style.item_spacing.x = spacing.x * 2.0;
        let offset_label = self.window().menu_columns.offset_label();
        let text_pos = Vec2 {
            x: self.window().cursor.x + offset_label,
            y: pos.y + base,
        };
        // `ImVec2(label_size.x, 0.0f)`: in a bar the row has no height of its own.
        let pressed = self.selectable_box(
            "",
            false,
            Vec2 {
                x: label_size.x,
                y: 0.0,
            },
            false,
            false,
            true,
        );
        self.style.item_spacing.x = spacing.x;
        let font = self.font.clone();
        let text_col = self.style.color_u32(crate::style::Col::Text);
        self.draw_list
            .add_text(&font, font.size, text_pos, text_col, shown);
        self.window_mut().cursor.x += (spacing.x * (-1.0 + 0.5)).trunc();
        pressed
    }

    /// `MenuItemEx` inside a vertical menu: a row that fills the available width, its label in the
    /// measured label column and its shortcut in the shortcut column, right-aligned.
    fn menu_item_column(&mut self, label: &str, shortcut: Option<&str>) -> bool {
        let shown = rendered_text(label);
        let label_size = self.font.calc_text_size(shown, self.font.size);
        let base = self.line_text_base_offset();
        let pos = self.window().cursor;
        let shortcut_width = shortcut
            .map(|text| self.font.calc_text_size(text, self.font.size).x)
            .unwrap_or(0.0);
        // `IM_TRUNC(FontSize * 1.20f)`: the mark column is always accounted for, even with no
        // selectable row, because `selected` has no neutral setting.
        let mark_width = crate::menu_columns::checkmark_width(self.font.size);
        let min_width = self.window_mut().menu_columns.decl_columns(
            0.0,
            label_size.x,
            shortcut_width,
            mark_width,
        );
        let columns = self.window().menu_columns;
        // `extra_w = ImMax(0.0f, GetContentRegionAvail().x - min_w)`: what is left of the row is
        // pushed to the right, which is what right-aligns the shortcut.
        let avail = self.content_region_max_x() - pos.x;
        let stretch = (avail - min_width).max(0.0);
        let pressed = self.selectable_box(
            "",
            false,
            Vec2 {
                x: min_width,
                y: label_size.y,
            },
            true,
            false,
            true,
        );
        let font = self.font.clone();
        let text_col = self.style.color_u32(crate::style::Col::Text);
        let text_pos = Vec2 {
            x: pos.x + columns.offset_label(),
            y: pos.y + base,
        };
        self.draw_list
            .add_text(&font, font.size, text_pos, text_col, shown);
        if shortcut_width > 0.0 {
            if let Some(shortcut) = shortcut {
                let colour = self.style.color_u32(crate::style::Col::TextDisabled);
                self.draw_list.add_text(
                    &font,
                    font.size,
                    Vec2 {
                        x: text_pos.x + columns.offset_shortcut() + stretch,
                        y: text_pos.y,
                    },
                    colour,
                    shortcut,
                );
            }
        }
        // Not modelled: an icon column, and the check mark of a selected row — `selected` is always
        // false here, so the reserved mark column stays empty.
        pressed
    }

    /// `ImGui::BeginMenu`: an entry of a menu bar, or a row of a vertical menu, whose own window
    /// holds the entries of the menu it opens. Returns true while that window is open, and
    /// [`Context::end_menu`] closes it.
    ///
    /// The menu window is a popup with `ImGuiWindowFlags_ChildMenu` named `"{label}###Menu_00"`,
    /// placed below the entry it belongs to. Like every popup it is measured on its first frame and
    /// draws nothing in it, so a menu settles on its second frame.
    pub fn begin_menu(&mut self, label: &str) -> bool {
        let id = self.get_id(label);
        // A row chosen on the previous frame closed the menu then; that frame still drew it.
        if self.popup_pending_close {
            self.popup_open = None;
            self.popup_pending_close = false;
            self.popup_rect = None;
            self.child_menu_host = None;
        }
        let horizontal = self.window().layout_horizontal;
        let shown = rendered_text(label);
        let label_size = self.font.calc_text_size(shown, self.font.size);
        let spacing = self.style.item_spacing;
        let base = self.line_text_base_offset();
        let frame_padding = self.style.frame_padding;
        let menu_bar_height = self.window().menu_bar_height;
        let pos = self.window().cursor;
        let mut menu_is_open = self.popup_open == Some(id);

        // `popup_pos`: where the child menu is anchored, before `FindBestWindowPosForPopup` moves it.
        let (pressed, popup_pos) = if horizontal {
            self.window_mut().cursor.x += (spacing.x * 0.5).trunc();
            self.style.item_spacing.x = spacing.x * 2.0;
            let offset_label = self.window().menu_columns.offset_label();
            let text_pos = Vec2 {
                x: self.window().cursor.x + offset_label,
                y: pos.y + base,
            };
            let pressed = self.selectable_box("", menu_is_open, label_size, false, true, false);
            self.style.item_spacing.x = spacing.x;
            let font = self.font.clone();
            let text_col = self.style.color_u32(crate::style::Col::Text);
            self.draw_list
                .add_text(&font, font.size, text_pos, text_col, shown);
            self.window_mut().cursor.x += (spacing.x * (-1.0 + 0.5)).trunc();
            let popup_pos = Vec2 {
                x: pos.x - 1.0 - (spacing.x * 0.5).trunc(),
                y: text_pos.y - frame_padding.y + menu_bar_height,
            };
            (pressed, popup_pos)
        } else {
            // A row of a vertical menu: the label column and the arrow of a submenu, which is why
            // the mark column is reserved here too.
            let mark_width = crate::menu_columns::checkmark_width(self.font.size);
            let min_width =
                self.window_mut()
                    .menu_columns
                    .decl_columns(0.0, label_size.x, 0.0, mark_width);
            let columns = self.window().menu_columns;
            let avail = self.content_region_max_x() - pos.x;
            let extra_w = (avail - min_width).max(0.0);
            let text_pos = Vec2 {
                x: pos.x,
                y: pos.y + base,
            };
            let pressed = self.selectable_box(
                "",
                menu_is_open,
                Vec2 {
                    x: min_width,
                    y: label_size.y,
                },
                true,
                true,
                false,
            );
            let font = self.font.clone();
            let text_col = self.style.color_u32(crate::style::Col::Text);
            self.draw_list.add_text(
                &font,
                font.size,
                Vec2 {
                    x: text_pos.x + columns.offset_label(),
                    y: text_pos.y,
                },
                text_col,
                shown,
            );
            self.draw_list.add_arrow(
                Vec2 {
                    x: text_pos.x + columns.offset_mark() + extra_w + self.font.size * 0.30,
                    y: text_pos.y,
                },
                text_col,
                ARROW_RIGHT_COS,
                ARROW_RIGHT_SIN,
                1.0,
                self.font.size,
                self.style.symbol_arrow_size_factor,
                self.style.symbol_arrow_thickness_factor,
            );
            let popup_pos = Vec2 {
                x: pos.x,
                y: text_pos.y - self.style.window_padding.y,
            };
            (pressed, popup_pos)
        };

        let hovered = self.last_item_hovered;
        let host = self.window().name.clone();
        // `IsRootOfOpenMenuSet`: while one of this window's menus is open, its siblings open on hover.
        let menuset_is_open = self.child_menu_host.as_deref() == Some(host.as_str());
        let mut want_open = false;
        let mut want_close = false;
        if horizontal {
            if menu_is_open && pressed && menuset_is_open {
                // Clicking an open menu again closes it.
                want_close = true;
                menu_is_open = false;
            } else if pressed || (hovered && menuset_is_open && !menu_is_open) {
                want_open = true;
            }
        } else {
            // Not modelled: the exemption that keeps the menu open while the mouse travels toward
            // the child menu (`moving_toward_child_menu`), and the hover-timer fallback that opens a
            // menu the mouse has rested on.
            if menu_is_open && !hovered && self.hovered_window.as_deref() == Some(host.as_str()) {
                want_close = true;
            }
            if !menu_is_open && (pressed || hovered) {
                want_open = true;
            }
        }
        if want_close && self.popup_open == Some(id) {
            self.popup_open = None;
            self.popup_rect = None;
            self.child_menu_host = None;
        }
        // Not modelled: when another menu of this level is open the reference opens this one on the
        // next frame instead, to close the sibling first.
        if want_open && !menu_is_open {
            self.open_popup(label);
            menu_is_open = true;
        }
        if !menu_is_open {
            return false;
        }
        self.begin_child_menu(label, popup_pos)
    }

    /// `BeginPopupMenuEx` for a menu: the child menu's own window, anchored below its entry.
    fn begin_child_menu(&mut self, label: &str, popup_pos: Vec2) -> bool {
        // `ImFormatString(name, "%s###Menu_%02d", label, g.BeginMenuDepth)`: recycled by depth. The
        // "###" keeps the identifier stable while the label in front of it is what gets displayed.
        let name = format!("{label}###Menu_{:02}", self.menu_depth);
        let measured = WindowMemory::find(&self.memory, &name).map(|memory| memory.size_full);
        self.popup_first_frame = measured.is_none();
        let size = measured.unwrap_or_default();
        // `FindBestWindowPosForPopup` for a child menu whose parent is a menu bar: the position the
        // caller asked for is clamped into the display, and then the menu is pushed below the bar.
        let max_x = self.io.display_size.x - size.x;
        let clamped_x = if popup_pos.x < 0.0 {
            0.0
        } else if popup_pos.x > max_x {
            max_x
        } else {
            popup_pos.x
        };
        let pos = Vec2 {
            x: clamped_x.max(0.0),
            y: self.menu_bar_clip_max_y.unwrap_or(popup_pos.y),
        };
        let flags = WindowFlags::ALWAYS_AUTO_RESIZE
            | WindowFlags::NO_RESIZE
            | WindowFlags::NO_MOVE
            | WindowFlags::NO_SCROLLBAR
            | WindowFlags::POPUP;
        self.set_next_window_flags(flags);
        // The menu belongs to the window whose entry opened it: that window takes the focus back
        // when a row is chosen, and it is the parent a nested submenu is placed against.
        let host = self.window().name.clone();
        self.popup_host = Some(host.clone());
        self.child_menu_host = Some(host);
        self.menu_depth += 1;
        let host_list = core::mem::take(&mut self.draw_list);
        self.list_stack.push(host_list);
        self.last_popup_name = Some(name.clone());
        self.begin_window(&name, Some(pos), None);
        true
    }

    /// `ImGui::EndMenu`: close the menu's window, handing the list back to the window that hosts it.
    pub fn end_menu(&mut self) {
        self.menu_depth = self.menu_depth.saturating_sub(1);
        self.end_popup();
    }

    /// `ImGui::BeginTooltip`: a tooltip is a window at the mouse that takes neither the mouse nor the
    /// keyboard, so nothing under it stops hovering.
    pub fn begin_tooltip(&mut self) {
        // `ImFormatString("##Tooltip_%02d")`: tooltips are recycled by depth.
        let name = "##Tooltip_00";
        // On its first frame a tooltip only measures its content, like a popup: the list it produced
        // is empty and never reaches the draw data.
        let measured = WindowMemory::find(&self.memory, name).map(|memory| memory.size_full);
        self.popup_first_frame = measured.is_none();
        // `TOOLTIP_DEFAULT_OFFSET_MOUSE`: down and to the right of the mouse.
        let pos = {
            let mouse = self.io.mouse_pos.unwrap_or_default();
            Vec2 {
                x: mouse.x + 25.0,
                y: mouse.y + 10.0,
            }
        };
        let flags = WindowFlags::ALWAYS_AUTO_RESIZE
            | WindowFlags::NO_RESIZE
            | WindowFlags::NO_MOVE
            | WindowFlags::NO_SCROLLBAR
            | WindowFlags::POPUP
            | WindowFlags::TOOLTIP;
        self.set_next_window_flags(flags);
        let host = core::mem::take(&mut self.draw_list);
        self.list_stack.push(host);
        self.begin_window(name, Some(pos), None);
    }

    /// `ImGui::EndTooltip`: close the tooltip window and hand the host its list back.
    pub fn end_tooltip(&mut self) {
        self.end_window();
        if self.popup_first_frame {
            self.frame_lists.pop();
            self.list_parents.pop();
            self.popup_first_frame = false;
        }
        self.draw_list = self
            .list_stack
            .pop()
            .unwrap_or_else(|| self.new_window_draw_list());
    }

    /// `ImGui::InputText`, single line: the framed field, its text, and the caret while the field
    /// owns the keyboard. Returns true when the value changed.
    ///
    /// Only the mouse half is modelled so far: a click activates the field and drops the caret where
    /// it landed (rounded to the nearest character boundary), a click elsewhere releases it, and the
    /// text is written back to the caller's buffer when that happens. Typing, the arrow keys, the
    /// selection and its keyboard-driven scroll are the next step.
    pub fn input_text(&mut self, label: &str, buffer: &mut String) -> bool {
        let id = self.get_id(label);
        let pad = self.style.frame_padding;
        let shown = rendered_text(label);
        let label_size = self.font.calc_text_size(shown, self.font.size);
        let frame_size = Vec2 {
            x: self.calc_item_width(),
            y: label_size.y + pad.y * 2.0,
        };
        let total = Vec2 {
            x: frame_size.x
                + if label_size.x > 0.0 {
                    self.style.item_inner_spacing.x + label_size.x
                } else {
                    0.0
                },
            y: frame_size.y,
        };
        let frame_min = self.window().cursor;
        let frame_max = Vec2 {
            x: frame_min.x + frame_size.x,
            y: frame_min.y + frame_size.y,
        };
        // `ItemSize(total_bb, FramePadding.y)` then `ItemAdd(total_bb, id, ..., Inputable)`.
        self.dummy(total);

        let hovered = self.hovered_in(frame_min, frame_max);
        if hovered {
            // `SetMouseCursor(ImGuiMouseCursor_TextInput)`.
            self.set_mouse_cursor(crate::io::MouseCursor::TextInput);
        }
        let user_clicked = hovered && self.io.mouse_clicked[0];
        // `g.ActiveId == id`: the field owns the keyboard. The state stays in the context after the
        // focus is released (the reference only replaces it when another field takes over), so the
        // frame that releases it can still draw its caret.
        let mut active = self.active_id == Some(id);
        let mut just_activated = false;
        if user_clicked && !active {
            // `Init state`: the field takes a copy of the caller's buffer. Coming back to the same
            // field recycles the cursor, as `recycle_state` does.
            let mut state = match self.input_text_state.take() {
                Some(state) if state.id == id && state.text == *buffer => {
                    let mut state = state;
                    state.cursor_clamp();
                    state
                }
                _ => crate::input_text::InputTextState::new(id, buffer),
            };
            state.cursor_anim_reset();
            // `state->TextToRevertTo`: what Escape goes back to, taken when the field takes over.
            state.revert_to = buffer.clone();
            // `state->Scroll = ImVec2(0, 0)`: a field that takes the keyboard starts unscrolled.
            state.scroll_x = 0.0;
            // `stb_textedit_initialize_state`: a new keyboard session starts with empty undo stacks.
            state.clear_undo();
            self.input_text_state = Some(state);
            self.active_id = Some(id);
            active = true;
            just_activated = true;
        }
        // `Release focus when we click outside`. The frame still draws as if active: the reference
        // clears the active id at the end of the function, after the render block.
        let mut release = active && self.io.mouse_clicked[0] && !user_clicked;
        // The mouse, in the reference's order: a double click selects the word, a click drops the
        // caret or (with Shift) extends the selection, and holding the button drags it.
        if active {
            let mouse = self.io.mouse_pos.unwrap_or_default();
            let scroll_x = self
                .input_text_state
                .as_ref()
                .map(|state| state.scroll_x)
                .unwrap_or(0.0);
            let mouse_x = mouse.x - frame_min.x - pad.x + scroll_x;
            let font = self.font.clone();
            let size = font.size;
            let double_clicked =
                hovered && self.io.mouse_clicked_count[0] >= 2 && !self.io.key_shift;
            let shift = self.io.key_shift;
            let clicked = self.io.mouse_clicked[0];
            let dragging = self.io.mouse_down[0]
                && (self.io.mouse_delta.x != 0.0 || self.io.mouse_delta.y != 0.0);
            if let Some(state) = self.input_text_state.as_mut() {
                if double_clicked {
                    state.double_click(mouse_x, &font, size);
                } else if clicked {
                    // The click branch consumes the frame even when the mouse is outside the field:
                    // `if (io.MouseClicked[0] && !SelectedAllMouseLock) { if (hovered) ... }`.
                    if hovered {
                        if shift {
                            state.drag(mouse_x, &font, size);
                        } else {
                            state.click(mouse_x, &font, size);
                        }
                        state.cursor_anim_reset();
                    }
                } else if dragging {
                    state.drag(mouse_x, &font, size);
                    state.cursor_anim_reset();
                    state.cursor_follow = true;
                }
            }
        }

        // The edit: `g.ActiveId == id && !g.ActiveIdIsJustActivated`. Characters first, then at most
        // one key, in the reference's priority order.
        let mut validated = false;
        // `render_cursor` is decided before the edit, and Escape turns it off for its own frame.
        let mut render_cursor = active;
        if active && !just_activated && !release {
            let shift = self.io.key_shift;
            let characters: Vec<char> = self.io.input_queue_characters.clone();
            for c in characters {
                // `InputTextFilterCharacter`: control characters and Delete are dropped, and Tab is
                // skipped by the caller of OnCharPressed.
                if c == '\t' || (c as u32) < 0x20 || (c as u32) == 0x7F {
                    continue;
                }
                if let Some(state) = self.input_text_state.as_mut() {
                    state.on_char_pressed(c);
                }
            }
            // `IsKeyPressed` with the default repeat, so a held arrow keeps moving.
            let io = &self.io;
            let key = if io.is_key_pressed_repeat(Key::LeftArrow) {
                Some(crate::input_text::EditKey::Left)
            } else if io.is_key_pressed_repeat(Key::RightArrow) {
                Some(crate::input_text::EditKey::Right)
            } else if io.is_key_pressed_repeat(Key::Home) {
                Some(crate::input_text::EditKey::Home)
            } else if io.is_key_pressed_repeat(Key::End) {
                Some(crate::input_text::EditKey::End)
            } else if io.is_key_pressed_repeat(Key::Delete) {
                Some(crate::input_text::EditKey::Delete)
            } else if io.is_key_pressed_repeat(Key::Backspace) {
                Some(crate::input_text::EditKey::Backspace)
            } else {
                None
            };
            let is_enter =
                io.is_key_pressed_repeat(Key::Enter) || io.is_key_pressed_repeat(Key::KeypadEnter);
            let is_escape = io.is_key_pressed_repeat(Key::Escape);
            // `Shortcut(ImGuiMod_Ctrl | key)`: the key with Control as its only modifier.
            let ctrl_mods = io.key_ctrl && !io.key_shift && !io.key_alt && !io.key_super;
            let ctrl_shift_mods = io.key_ctrl && io.key_shift && !io.key_alt && !io.key_super;
            let shortcut = if !ctrl_mods && !ctrl_shift_mods {
                Shortcut::None
            } else if io.is_key_pressed(Key::A) {
                Shortcut::SelectAll
            } else if io.is_key_pressed(Key::C) {
                Shortcut::Copy
            } else if io.is_key_pressed_repeat(Key::X) {
                Shortcut::Cut
            } else if io.is_key_pressed_repeat(Key::V) {
                Shortcut::Paste
            } else if !ctrl_shift_mods && io.is_key_pressed_repeat(Key::Z) {
                Shortcut::Undo
            } else if io.is_key_pressed_repeat(Key::Y) || io.is_key_pressed_repeat(Key::Z) {
                Shortcut::Redo
            } else {
                Shortcut::None
            };

            if let Some(key) = key {
                if let Some(state) = self.input_text_state.as_mut() {
                    state.on_key(key, shift);
                }
            } else if is_enter {
                // `validated = clear_active_id = true`: the value is applied on this frame.
                validated = true;
                release = true;
            } else if is_escape {
                // Escape goes back to what the field held when it took the keyboard, and the frame
                // it does that draws neither the caret nor the selection.
                if let Some(state) = self.input_text_state.as_mut() {
                    let revert = state.revert_to.clone();
                    state.replace_text(&revert);
                }
                release = true;
                render_cursor = false;
            } else if shortcut == Shortcut::Undo || shortcut == Shortcut::Redo {
                // `state->ClearSelection(); state->CursorFollow = true;` then the stack operation.
                if let Some(state) = self.input_text_state.as_mut() {
                    state.select_start = state.cursor;
                    state.select_end = state.cursor;
                    state.cursor_follow = true;
                    if shortcut == Shortcut::Undo {
                        state.undo();
                    } else {
                        state.redo();
                    }
                }
            } else if shortcut == Shortcut::SelectAll {
                // `state->SelectAll()` and let the view follow the caret.
                if let Some(state) = self.input_text_state.as_mut() {
                    state.select_all();
                    state.cursor_follow = true;
                }
            } else if shortcut == Shortcut::Copy || shortcut == Shortcut::Cut {
                // The selection goes to the clipboard; with nothing selected, the whole text does.
                let copied = self.input_text_state.as_ref().map(|state| {
                    let (begin, end) = if state.has_selection() {
                        (
                            state.select_start.min(state.select_end),
                            state.select_start.max(state.select_end),
                        )
                    } else {
                        (0, state.text.len())
                    };
                    state.text[begin..end].to_string()
                });
                if let Some(copied) = copied {
                    self.io.set_clipboard_text(&copied);
                }
                if shortcut == Shortcut::Cut {
                    if let Some(state) = self.input_text_state.as_mut() {
                        state.cut_selection();
                    }
                }
            } else if shortcut == Shortcut::Paste {
                if let Some(text) = self.io.get_clipboard_text().map(str::to_string) {
                    // `InputTextFilterCharacter` on the pasted text: control characters dropped.
                    let filtered: String = text
                        .chars()
                        .filter(|c| !((*c as u32) < 0x20 || (*c as u32) == 0x7F))
                        .collect();
                    if let Some(state) = self.input_text_state.as_mut() {
                        state.paste(&filtered);
                    }
                }
            }
        }

        // `ImGuiItemFlags_LiveEditOnInputText` is on by default, so the working copy is written to
        // the caller's buffer as soon as it differs, and the widget reports the change that frame.
        let mut value_changed = false;
        if active {
            if let Some(state) = self.input_text_state.as_ref() {
                if state.text != *buffer {
                    *buffer = state.text.clone();
                    value_changed = true;
                }
            }
        }

        // Render. `RenderFrame(frame_bb.Min, frame_bb.Max, FrameBg, true, FrameRounding)`: with the
        // default style there is no border and no rounding, so it is one filled rectangle.
        self.draw_list.prim_rect(
            frame_min,
            frame_max,
            self.style.color_u32(crate::style::Col::FrameBg),
        );
        if self.style.frame_border_size > 0.0 {
            self.draw_list.add_aligned_rect_border(
                frame_min,
                frame_max,
                self.style.color_u32(crate::style::Col::Border),
                self.style.frame_border_size,
            );
        }
        // `draw_pos = frame_bb.Min + FramePadding`, and the clip is the frame itself.
        let draw_pos = Vec2 {
            x: frame_min.x + pad.x,
            y: frame_min.y + pad.y,
        };
        let clip = (frame_min, frame_max);
        let (text, cursor, cursor_anim, mut scroll_x, cursor_follow) =
            match (&self.input_text_state, active) {
                (Some(state), true) => (
                    state.text.clone(),
                    state.cursor,
                    state.cursor_anim,
                    state.scroll_x,
                    state.cursor_follow,
                ),
                _ => (buffer.clone(), 0, 0.0, 0.0, false),
            };
        let font = self.font.clone();
        let cursor = cursor.min(text.len());
        // `cursor_offset.x`: the width of the text before the caret, rounded up a whole pixel.
        let offset_x = (font.calc_text_size(&text[..cursor], font.size).x + 0.999_99).trunc();
        // `if (render_cursor && state->CursorFollow)`: only the keyboard moves the scroll, in steps of
        // a quarter of the field's width, and it catches the caret up on the frame it moved.
        if render_cursor && cursor_follow {
            let inner_width = frame_size.x;
            let scroll_increment_x = inner_width * 0.25;
            let visible_width = inner_width - pad.x;
            if offset_x < scroll_x {
                scroll_x = (offset_x - scroll_increment_x).max(0.0).trunc();
            } else if offset_x - visible_width >= scroll_x {
                scroll_x = (offset_x - visible_width + scroll_increment_x).trunc();
            }
            if let Some(state) = self.input_text_state.as_mut() {
                state.scroll_x = scroll_x;
                state.cursor_follow = false;
            }
        }
        let scroll_x = if render_cursor { scroll_x } else { 0.0 };
        let text_x = draw_pos.x - scroll_x;
        // The selection, drawn before the text: `TextSelectedBg` from `draw_pos.y - 1` to
        // `draw_pos.y + 2 + FontSize`, as wide as the selected characters.
        let selection = match (&self.input_text_state, render_cursor) {
            (Some(state), true) if state.has_selection() => {
                let begin = state.select_start.min(state.select_end).min(text.len());
                let end = state.select_start.max(state.select_end).min(text.len());
                let begin = if text.is_char_boundary(begin) {
                    begin
                } else {
                    0
                };
                let end = if text.is_char_boundary(end) {
                    end
                } else {
                    text.len()
                };
                Some((begin, end))
            }
            _ => None,
        };
        if let Some((begin, end)) = selection {
            let before = (font.calc_text_size(&text[..begin], font.size).x + 0.999_99).trunc();
            let width = (font.calc_text_size(&text[begin..end], font.size).x + 0.999_99).trunc();
            let rect = (
                Vec2 {
                    x: text_x + before,
                    y: draw_pos.y - 1.0,
                },
                Vec2 {
                    x: text_x + before + width,
                    y: draw_pos.y + 2.0 + font.size,
                },
            );
            // `rect.ClipWith(clip_rect)`.
            let clipped = (
                Vec2 {
                    x: rect.0.x.max(clip.0.x),
                    y: rect.0.y.max(clip.0.y),
                },
                Vec2 {
                    x: rect.1.x.min(clip.1.x),
                    y: rect.1.y.min(clip.1.y),
                },
            );
            if clipped.0.x < clipped.1.x && clipped.0.y < clipped.1.y {
                self.draw_list.prim_rect(
                    clipped.0,
                    clipped.1,
                    self.style.color_u32(crate::style::Col::TextSelectedBg),
                );
            }
        }
        let text_col = self.style.color_u32(crate::style::Col::Text);
        let text_pos = Vec2 {
            x: text_x,
            y: draw_pos.y,
        };
        self.draw_list
            .add_text_clipped(&font, font.size, text_pos, text_col, &text, clip);

        // The caret: `trunc(draw_pos + cursor_offset - draw_scroll)`, one pixel wide, from
        // `y - FontSize + 1` to `y - 1`.
        if render_cursor {
            // `state->CursorAnim += io.DeltaTime`, then the blink rule.
            let anim = cursor_anim + self.io.delta_time;
            if let Some(state) = self.input_text_state.as_mut() {
                state.cursor_anim = anim;
            }
            let visible =
                !self.io.config_input_text_cursor_blink || anim <= 0.0 || (anim % 1.20) <= 0.80;
            let caret_x = (text_x + offset_x).trunc();
            let caret_y = draw_pos.y + font.size;
            let caret = (
                Vec2 {
                    x: caret_x,
                    y: caret_y - font.size + 1.0,
                },
                Vec2 {
                    x: caret_x + 1.0,
                    y: caret_y - 1.0,
                },
            );
            let overlaps = caret.0.x <= clip.1.x
                && caret.1.x >= clip.0.x
                && caret.0.y <= clip.1.y
                && caret.1.y >= clip.0.y;
            if visible && overlaps {
                self.draw_list.prim_rect(
                    caret.0,
                    caret.1,
                    self.style.color_u32(crate::style::Col::InputTextCursor),
                );
            }
        }

        // `ClearActiveID()` after the value is applied but before the frame is drawn: the caret is
        // still drawn because `render_cursor` was decided earlier. The state itself stays until
        // another field takes it.
        if active && (validated || release) && self.active_id == Some(id) {
            self.active_id = None;
        }
        value_changed
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
