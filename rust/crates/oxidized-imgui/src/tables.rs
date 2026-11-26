//! `ImGuiTable`: the layout of a table with fixed columns.
//!
//! The paths the table goldens pin are here: fixed and stretch columns, rows that grow to their
//! contents, the cell padding and spacing the reference derives from `ImGuiStyle::CellPadding`, the
//! display order the reorder drag rewrites, and the sort orders and directions the headers draw and
//! the clicks change. Freezing, the table's own scrolling and the context menu are still missing;
//! the geometry they add is measured in `docs/TRASPASO.md`.

use crate::draw_list::Vec2;

/// `ImGuiTableFlags`: the table bits the layout reads.
pub mod table_flags {
    /// `ImGuiTableFlags_BordersInnerH`: horizontal borders between rows.
    pub const BORDERS_INNER_H: u32 = 1 << 7;
    /// `ImGuiTableFlags_BordersOuterH`: horizontal borders above and below the table.
    pub const BORDERS_OUTER_H: u32 = 1 << 8;
    /// `ImGuiTableFlags_BordersInnerV`: vertical borders between columns.
    pub const BORDERS_INNER_V: u32 = 1 << 9;
    /// `ImGuiTableFlags_BordersOuterV`: vertical borders on the sides.
    pub const BORDERS_OUTER_V: u32 = 1 << 10;
    /// `ImGuiTableFlags_PadOuterX`.
    pub const PAD_OUTER_X: u32 = 1 << 21;
    /// `ImGuiTableFlags_NoPadOuterX`.
    pub const NO_PAD_OUTER_X: u32 = 1 << 22;
    /// `ImGuiTableFlags_NoPadInnerX`.
    pub const NO_PAD_INNER_X: u32 = 1 << 23;
    /// `ImGuiTableFlags_Resizable`: columns can be resized, so a fixed one keeps its request
    /// instead of tracking its contents.
    pub const RESIZABLE: u32 = 1 << 0;
    /// `ImGuiTableFlags_Reorderable`: the user can drag a header to reorder the columns.
    pub const REORDERABLE: u32 = 1 << 1;
    /// `ImGuiTableFlags_Sortable`: the table exposes sort specs.
    pub const SORTABLE: u32 = 1 << 3;
    /// `ImGuiTableFlags_SortMulti`: the specs may hold more than one column.
    pub const SORT_MULTI: u32 = 1 << 26;
    /// `ImGuiTableFlags_SortTristate`: the specs may end up empty.
    pub const SORT_TRISTATE: u32 = 1 << 27;
    /// `ImGuiTableFlags_RowBg`: every other row gets `TableRowBgAlt`.
    pub const ROW_BG: u32 = 1 << 6;
    /// `ImGuiTableFlags_ScrollX`: the table builds its rows in an inner scrolling window.
    pub const SCROLL_X: u32 = 1 << 24;
    /// `ImGuiTableFlags_ScrollY`: the table builds its rows in an inner scrolling window.
    pub const SCROLL_Y: u32 = 1 << 25;
    /// `ImGuiTableFlags_SizingMask_`: the bits that pick the table sizing policy.
    pub const SIZING_MASK: u32 = 7 << 13;
    /// `ImGuiTableFlags_SizingFixedFit`: the default policy.
    pub const SIZING_FIXED_FIT: u32 = 1 << 13;
    /// `ImGuiTableFlags_SizingFixedSame`.
    pub const SIZING_FIXED_SAME: u32 = 2 << 13;
    /// `ImGuiTableFlags_SizingStretchProp`: stretch columns weight by their contents.
    pub const SIZING_STRETCH_PROP: u32 = 3 << 13;
    /// `ImGuiTableFlags_SizingStretchSame`: every column stretches with the same weight.
    pub const SIZING_STRETCH_SAME: u32 = 4 << 13;
    /// `ImGuiTableFlags_PreciseWidths`: keeps the rounding remainder out of the stretch columns.
    pub const PRECISE_WIDTHS: u32 = 1 << 19;
    /// `ImGuiTableRowFlags_Headers`: the row holds column headers.
    pub const ROW_HEADERS: u32 = 1 << 0;
}

/// `ImGuiTableColumnFlags`: the column bits the layout reads.
pub mod column_flags {
    /// `ImGuiTableColumnFlags_WidthFixed`: the column keeps the width it asked for.
    pub const WIDTH_FIXED: u32 = 1 << 4;
    /// `ImGuiTableColumnFlags_WidthStretch`: the column takes a share of what the fixed ones leave.
    pub const WIDTH_STRETCH: u32 = 1 << 3;
    /// `ImGuiTableColumnFlags_WidthMask_`.
    pub const WIDTH_MASK: u32 = 1 << 3 | 1 << 4;
    /// `ImGuiTableColumnFlags_DefaultSort`: the column starts as the sorting one.
    pub const DEFAULT_SORT: u32 = 1 << 2;
    /// `ImGuiTableColumnFlags_NoSort`.
    pub const NO_SORT: u32 = 1 << 9;
    /// `ImGuiTableColumnFlags_NoSortAscending`: the column cannot be sorted ascending.
    pub const NO_SORT_ASCENDING: u32 = 1 << 10;
    /// `ImGuiTableColumnFlags_NoSortDescending`: the column cannot be sorted descending.
    pub const NO_SORT_DESCENDING: u32 = 1 << 11;
    /// `ImGuiTableColumnFlags_PreferSortDescending`: the direction its `DefaultSort` starts with.
    pub const PREFER_SORT_DESCENDING: u32 = 1 << 15;
    /// `ImGuiTableColumnFlags_NoResize`.
    pub const NO_RESIZE: u32 = 1 << 5;
    /// `ImGuiTableColumnFlags_NoReorder`: other columns cannot cross over it.
    pub const NO_REORDER: u32 = 1 << 6;
    /// `ImGuiTableColumnFlags_IndentEnable`: column 0 keeps the current indent.
    pub const INDENT_ENABLE: u32 = 1 << 16;
    /// `ImGuiTableColumnFlags_IndentDisable`: the other columns ignore it.
    pub const INDENT_DISABLE: u32 = 1 << 17;
    /// `ImGuiTableColumnFlags_IndentMask_`.
    pub const INDENT_MASK: u32 = 1 << 16 | 1 << 17;
    /// `ImGuiTableColumnFlags_IsEnabled`: a status bit the layout sets on the columns it processes.
    pub const IS_ENABLED: u32 = 1 << 24;
    /// `ImGuiTableColumnFlags_IsVisible`.
    pub const IS_VISIBLE: u32 = 1 << 25;
    /// `ImGuiTableColumnFlags_IsSorted`: the column a sort spec points at.
    pub const IS_SORTED: u32 = 1 << 26;
    /// `ImGuiTableColumnFlags_IsHovered`: the column under the mouse.
    pub const IS_HOVERED: u32 = 1 << 27;
    /// `ImGuiTableColumnFlags_StatusMask_`, as far as this port sets it.
    pub const STATUS_MASK: u32 = 1 << 24 | 1 << 25 | 1 << 26 | 1 << 27;
}

/// `TABLE_BORDER_SIZE`.
const BORDER_SIZE: f32 = 1.0;

/// The sort state a table keeps between frames, `ImGuiTable`'s sort fields that the reference keeps
/// alive in the table it reuses (and reloads from its settings).
#[derive(Clone, Debug, Default)]
pub struct TableSortState {
    /// `column->SortOrder` of every column.
    pub orders: Vec<i32>,
    /// `column->SortDirection` of every column.
    pub directions: Vec<i32>,
    /// `table->IsSortSpecsDirty`: the specs have to be rebuilt and the caller told to sort.
    pub dirty: bool,
    /// `table->SortSpecs.SpecsDirty`: stays set until the caller clears it.
    pub specs_dirty: bool,
}

/// One column, `ImGuiTableColumn`, with the geometry `TableUpdateLayout` resolves for it.
#[derive(Clone, Debug, Default)]
pub struct TableColumn {
    /// `column->NameOffset` resolved: the label `TableSetupColumn` was given.
    pub name: String,
    pub flags: u32,
    /// `column->WidthRequest`: what the layout will give the column.
    pub width_request: f32,
    /// `column->InitStretchWeightOrWidth`: what `TableSetupColumn` asked for, kept apart because the
    /// request is rewritten every frame.
    pub init_width: f32,
    /// `column->WidthAuto`: the width the column's contents reached on the previous frame.
    pub width_auto: f32,
    /// `column->WidthGiven`: what the layout gave it.
    pub width_given: f32,
    /// `column->StretchWeight`: the share of the leftover width a stretch column asks for.
    pub stretch_weight: f32,
    /// `column->MinX`: where the column starts, padding included.
    pub min_x: f32,
    /// `column->MaxX`: where the next column starts.
    pub max_x: f32,
    /// `column->WorkMinX`: where the cell content starts.
    pub work_min_x: f32,
    /// `column->WorkMaxX`: where the cell content ends.
    pub work_max_x: f32,
    /// `column->ClipRect`: the span clipped to the table's inner clip.
    pub clip: [f32; 4],
    /// `column->ContentMaxXUnfrozen`: the furthest the column's content reached this frame, which
    /// decides whether the column can share a draw command with the others.
    pub content_max_x: f32,
    /// How wide the column's content turned out to be, kept for the next frame's width: the
    /// reference derives it from the extents it keeps in the reused `ImGuiTable`.
    pub content_width: f32,
    /// `column->NextEnabledColumn`: the next visible column, `-1` for the last one.
    pub next_enabled_column: i32,
}

/// The state of the table being built, `ImGuiTable`, reduced to what the first cut needs.
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub flags: u32,
    pub columns: Vec<TableColumn>,
    /// `table->OuterRect`: the rectangle `BeginTable` reserved at the cursor. Its bottom grows with
    /// the rows and is final only in `EndTable`.
    pub outer_rect: (Vec2, Vec2),
    /// `table->InnerClipRect`: the table's rectangle clipped to the window.
    pub inner_clip: (Vec2, Vec2),
    /// `table->BorderX1` / `BorderX2`: the horizontal span of the inner horizontal borders.
    pub border_x1: f32,
    pub border_x2: f32,
    /// `table->CellPaddingX`: only set when the columns are separated by borders.
    pub cell_padding_x: f32,
    /// `table->CellSpacingX1` / `CellSpacingX2`: the gaps on each side of a column.
    pub cell_spacing_x1: f32,
    pub cell_spacing_x2: f32,
    /// `table->OuterPaddingX`: the outer padding minus the cell padding, kept as the reference does.
    pub outer_padding_x: f32,
    /// `table->RowCellPaddingY`: `CellPadding.y` for the current row.
    pub row_cell_padding_y: f32,
    /// `table->RowPosY1` / `RowPosY2`: the row being built.
    pub row_pos_y1: f32,
    pub row_pos_y2: f32,
    /// `table->CurrentRow`: -1 until the first row starts.
    pub current_row: i32,
    /// `table->CurrentColumn`: -1 until the first cell starts.
    pub current_column: i32,
    /// `table->IsInsideRow`.
    pub is_inside_row: bool,
    /// `table->IsLayoutLocked`: the columns are resolved once per table.
    pub layout_locked: bool,
    /// `table->DeclColumnsCount`: how many `TableSetupColumn` calls the layout may use.
    pub decl_columns: usize,
    /// `table->RowFlags` and `table->RowMinHeight` of the row being built.
    pub row_flags: u32,
    /// `table->LastRowFlags`: the flags of the row closed most recently, which decide the colour of
    /// the border above the next one.
    pub last_row_flags: u32,
    pub row_min_height: f32,
    /// `table->RowBgColor[0]`: the background the row paints before the next one starts.
    pub row_bg0: Option<u32>,
    /// `table->RowCellData`: the per-cell backgrounds of the row being built, in submission order.
    pub row_cells: Vec<(usize, u32)>,
    /// `table->RowBgColorCounter`: the odd/even counter of `RowBg`, which skips header rows.
    pub row_bg_counter: u32,
    /// `table->SortSpecs.Specs`: the columns the caller should sort by, with their direction.
    pub sort_specs: Vec<(usize, i32)>,
    /// `table->IsSortSpecsDirty`: set when the specs changed and cleared when they are handed out.
    pub sort_specs_dirty: bool,
    /// `table->SortSpecs.SpecsDirty`: what the caller reads. It is set whenever the specs are rebuilt
    /// and stays set until the caller clears it, exactly as the reference documents.
    pub sort_specs_public_dirty: bool,
    /// `column->SortOrder`: where the column sits in the specs, `-1` when it is not sorted.
    pub sort_orders: Vec<i32>,
    /// `column->SortDirection`: `0` none, `1` ascending, `2` descending.
    pub sort_directions: Vec<i32>,
    /// True once the sort state came from a previous frame (or from the table's settings) instead of
    /// from the `DefaultSort` flags just declared.
    pub sort_state_loaded: bool,
    /// `column->DisplayOrder`: where each column sits in the display order, by column index.
    pub display_orders: Vec<i32>,
    /// `table->DisplayOrderToIndex`: the column index at each display position.
    pub display_order_to_index: Vec<i32>,
    /// `table->ReorderColumn`: the column being dragged, `-1` when none is.
    pub reorder_column: i32,
    /// `table->ReorderColumnDstOrder`: the display position the drag asked for, applied at the next
    /// `BeginTable`.
    pub reorder_column_dst_order: i32,
    /// `table->HoveredColumnBody`: the column under the mouse, `-1` outside the table and the column
    /// count in the unused space right of the last one.
    pub hovered_column_body: i32,
    /// `table_instance->HoveredRowLast`: the hovered row, one frame behind.
    pub hovered_row_last: i32,
    /// `table_instance->HoveredRowNext`: the row under the mouse on this frame.
    pub hovered_row_next: i32,
    /// `table->WorkRect`: the table's work rectangle, which is the outer one in a plain table.
    pub work_rect: (Vec2, Vec2),
    /// `table->BgClipRect`: where the row backgrounds are clipped, `InnerClipRect` here.
    pub bg_clip: (Vec2, Vec2),
}

impl Table {
    /// `TableBeginInitMemory` plus what `BeginTableEx` leaves in the table before the columns are
    /// resolved.
    pub fn new(flags: u32, columns: usize, outer_rect: (Vec2, Vec2)) -> Self {
        Self {
            flags,
            columns: vec![TableColumn::default(); columns],
            outer_rect,
            current_row: -1,
            current_column: -1,
            hovered_column_body: -1,
            hovered_row_last: 0,
            hovered_row_next: 0,
            sort_orders: vec![-1; columns],
            sort_directions: vec![0; columns],
            display_orders: (0..columns as i32).collect(),
            display_order_to_index: (0..columns as i32).collect(),
            reorder_column: -1,
            reorder_column_dst_order: -1,
            ..Self::default()
        }
    }

    /// `ImGui::TableSetupColumn`: the columns are declared in order, before the first row.
    pub fn setup_column(&mut self, label: &str, flags: u32, width: f32) {
        let index = self.decl_columns;
        if index >= self.columns.len() {
            return;
        }
        // `TableSetupColumnFlags`: the policy fills in a width kind, a table that cannot be resized
        // disables resizing, and the first column keeps the indent while the rest ignore it.
        let policy = self.flags & table_flags::SIZING_MASK;
        let mut flags = flags;
        if flags & column_flags::WIDTH_MASK == 0 {
            flags |= if policy == table_flags::SIZING_FIXED_FIT
                || policy == table_flags::SIZING_FIXED_SAME
            {
                column_flags::WIDTH_FIXED
            } else {
                column_flags::WIDTH_STRETCH
            };
        }
        if self.flags & table_flags::RESIZABLE == 0 {
            flags |= column_flags::NO_RESIZE;
        }
        if flags & column_flags::INDENT_MASK == 0 {
            flags |= if index == 0 {
                column_flags::INDENT_ENABLE
            } else {
                column_flags::INDENT_DISABLE
            };
        }
        self.columns[index] = TableColumn {
            name: label.to_string(),
            flags,
            width_request: width,
            init_width: width,
            ..TableColumn::default()
        };
        // `TableInitColumnDefaults` for the sort bits, which the reference applies when the column is
        // created (or its settings loaded): a `DefaultSort` column starts at sort order 0 — several of
        // them are linearized when the specs are built — and its direction comes from
        // `PreferSortDescending`. A column that is not sorted keeps `-1`.
        if !self.sort_state_loaded && index < self.sort_orders.len() {
            self.sort_orders[index] = if flags & column_flags::DEFAULT_SORT != 0 {
                0
            } else {
                -1
            };
            self.sort_directions[index] = if flags & column_flags::DEFAULT_SORT != 0 {
                if flags & column_flags::PREFER_SORT_DESCENDING != 0 {
                    2
                } else {
                    1
                }
            } else {
                0
            };
        }
        self.decl_columns += 1;
    }

    /// `TableSortSpecsSanitize` plus `TableSortSpecsBuild`: turns the columns' `SortOrder` values into
    /// the public specs, in sort order.
    ///
    /// The sanitize step closes the gaps and duplicates the orders may have — several `DefaultSort`
    /// columns start at order 0 — and keeps a single one unless `SortMulti` allows more. When no
    /// column is sorted, the first sortable one takes order 0, so a sortable table always starts with
    /// one spec (the rule the `tables_sort_tristate` golden pins).
    pub fn build_sort_specs(&mut self) {
        if self.flags & table_flags::SORTABLE == 0 {
            self.sort_specs.clear();
            self.sort_specs_dirty = false;
            return;
        }
        // `IsSortSpecsDirty` survives the rebuild: it comes from `BeginTable` on the first frame and
        // from a click, and only `TableGetSortSpecs` clears it. The one case that re-arms it here is
        // the reference's own test at the end of `TableUpdateLayout`: a sortable table with no specs
        // and no `SortTristate` must ask the caller to sort.
        let was_dirty = self.sort_specs_dirty;
        if was_dirty {
            self.sort_specs_public_dirty = true;
        }
        let columns = self.columns.len();
        let mut sort_order_count = 0u32;
        let mut sort_order_mask = 0u64;
        for n in 0..columns {
            let order = self.sort_orders.get(n).copied().unwrap_or(-1);
            if order == -1 {
                continue;
            }
            sort_order_count += 1;
            if (0..64).contains(&order) {
                sort_order_mask |= 1u64 << order;
            }
        }
        let need_fix_linearize = (1u64 << sort_order_count.min(63)) != sort_order_mask + 1;
        let need_fix_single_sort_order =
            sort_order_count > 1 && self.flags & table_flags::SORT_MULTI == 0;
        if need_fix_linearize || need_fix_single_sort_order {
            let mut fixed_mask = 0u64;
            for sort_n in 0..sort_order_count {
                let mut smallest: Option<usize> = None;
                for n in 0..columns {
                    if fixed_mask & (1u64 << n) != 0
                        || self.sort_orders.get(n).copied().unwrap_or(-1) == -1
                    {
                        continue;
                    }
                    if smallest.is_none_or(|s| {
                        self.sort_orders[n] < self.sort_orders.get(s).copied().unwrap_or(-1)
                    }) {
                        smallest = Some(n);
                    }
                }
                let Some(column_n) = smallest else { break };
                fixed_mask |= 1u64 << column_n;
                self.sort_orders[column_n] = sort_n as i32;
                if need_fix_single_sort_order {
                    for (n, order) in self.sort_orders.iter_mut().enumerate().take(columns) {
                        if n != column_n {
                            *order = -1;
                        }
                    }
                    break;
                }
            }
        }
        // `TableSortSpecsSanitize`'s fallback: the first sortable column takes the only order.
        let mut count = self
            .sort_orders
            .iter()
            .take(columns)
            .filter(|order| **order != -1)
            .count();
        if count == 0 {
            if let Some(n) = self
                .columns
                .iter()
                .position(|column| column.flags & column_flags::NO_SORT == 0)
            {
                self.sort_orders[n] = 0;
                self.sort_directions[n] = 1;
                count = 1;
            }
        }
        // `TableSortSpecsBuild`: one entry per sorted column, indexed by its sort order.
        let mut specs: Vec<(usize, i32)> = Vec::with_capacity(count);
        for n in 0..columns {
            let order = self.sort_orders.get(n).copied().unwrap_or(-1);
            if order == -1 || order as usize >= count {
                continue;
            }
            let direction = self.sort_directions.get(n).copied().unwrap_or(0);
            if specs.len() <= order as usize {
                specs.resize(order as usize + 1, (n, direction));
            } else {
                specs[order as usize] = (n, direction);
            }
        }
        self.sort_specs = specs;
        self.sort_specs_dirty = was_dirty
            || (self.sort_specs.is_empty() && self.flags & table_flags::SORT_TRISTATE == 0);
    }

    /// `column->SortOrder`: where the column sits in the specs, `-1` when it is not sorted.
    pub fn sort_order(&self, column: usize) -> i32 {
        self.sort_orders.get(column).copied().unwrap_or(-1)
    }

    /// The sort state that has to outlive the frame, `TableSaveSettings`'s sort fields.
    pub fn sort_state(&self) -> TableSortState {
        TableSortState {
            orders: self.sort_orders.clone(),
            directions: self.sort_directions.clone(),
            dirty: self.sort_specs_dirty,
            specs_dirty: self.sort_specs_public_dirty,
        }
    }

    /// Restores the state [`Self::sort_state`] saved, so the `DefaultSort` flags only pick the state
    /// of a table that has none.
    pub fn restore_sort_state(&mut self, state: &TableSortState) -> bool {
        if state.orders.len() != self.columns.len() || state.directions.len() != self.columns.len()
        {
            return false;
        }
        self.sort_orders.clone_from(&state.orders);
        self.sort_directions.clone_from(&state.directions);
        self.sort_specs_dirty = state.dirty;
        self.sort_specs_public_dirty = state.specs_dirty;
        self.sort_state_loaded = true;
        true
    }

    /// `column->SortDirection`: `0` none, `1` ascending, `2` descending.
    pub fn sort_direction(&self, column: usize) -> i32 {
        self.sort_directions.get(column).copied().unwrap_or(0)
    }

    /// `column->SortDirectionsAvailList`: the directions clicking the column cycles through, in order.
    /// `PreferSortDescending` moves descending to the front; `SortTristate` appends "none".
    pub fn sort_directions_avail(&self, column: usize) -> Vec<i32> {
        let flags = self
            .columns
            .get(column)
            .map(|entry| entry.flags)
            .unwrap_or(0);
        let no_ascending = flags & column_flags::NO_SORT_ASCENDING != 0;
        let no_descending = flags & column_flags::NO_SORT_DESCENDING != 0;
        let prefer_descending = flags & column_flags::PREFER_SORT_DESCENDING != 0;
        let mut avail = Vec::new();
        if !prefer_descending && !no_ascending {
            avail.push(1);
        }
        if prefer_descending && !no_descending {
            avail.push(2);
        }
        if prefer_descending && !no_ascending {
            avail.push(1);
        }
        if !prefer_descending && !no_descending {
            avail.push(2);
        }
        if self.flags & table_flags::SORT_TRISTATE != 0 || avail.is_empty() {
            avail.push(0);
        }
        avail
    }

    /// `TableGetColumnNextSortDirection`: the direction a click on the column would set. A column that
    /// is not sorted takes the first available direction; the sorted one takes the next in the list.
    pub fn next_sort_direction(&self, column: usize) -> i32 {
        let avail = self.sort_directions_avail(column);
        if avail.is_empty() {
            return 0;
        }
        if self.sort_order(column) == -1 {
            return avail[0];
        }
        let current = self.sort_direction(column);
        match avail.iter().position(|direction| *direction == current) {
            Some(at) => avail[(at + 1) % avail.len()],
            None => 0,
        }
    }

    /// `TableSetColumnSortDirection`: applies the direction a click produced. Unless `append` (which
    /// needs `SortMulti` and Shift), the other columns lose their sort order and this one takes 0.
    pub fn set_column_sort_direction(&mut self, column: usize, direction: i32, append: bool) {
        let append = append && self.flags & table_flags::SORT_MULTI != 0;
        let columns = self.columns.len();
        if column >= columns {
            return;
        }
        let mut sort_order_max = 0;
        if append {
            for n in 0..columns {
                sort_order_max = sort_order_max.max(self.sort_order(n));
            }
        }
        self.sort_directions[column] = direction;
        if direction == 0 {
            self.sort_orders[column] = -1;
        } else if self.sort_order(column) == -1 || !append {
            self.sort_orders[column] = if append { sort_order_max + 1 } else { 0 };
        }
        if !append {
            for (n, order) in self.sort_orders.iter_mut().enumerate().take(columns) {
                if n != column {
                    *order = -1;
                }
            }
        }
        self.sort_specs_dirty = true;
    }

    /// `column->DisplayOrder`: where the column sits in the display order.
    pub fn display_order(&self, column: usize) -> i32 {
        self.display_orders.get(column).copied().unwrap_or(-1)
    }

    /// `table->DisplayOrderToIndex`: the column at a display position, `-1` when there is none.
    pub fn column_at_display_order(&self, order: i32) -> i32 {
        if order < 0 {
            return -1;
        }
        self.display_order_to_index
            .get(order as usize)
            .copied()
            .unwrap_or(-1)
    }

    /// `TableFixDisplayOrder`: rebuilds the display-order-to-index map from the columns' orders.
    pub fn fix_display_order(&mut self) {
        for (n, order) in self.display_orders.iter().enumerate() {
            if let Some(slot) = self.display_order_to_index.get_mut(*order as usize) {
                *slot = n as i32;
            }
        }
    }

    /// `TableSetColumnDisplayOrder`: moves one column to `dst_order` and shifts the columns in between,
    /// then rebuilds the map, as the reference does.
    pub fn set_column_display_order(&mut self, column: usize, dst_order: i32) {
        let Some(&src_order) = self.display_orders.get(column) else {
            return;
        };
        let columns_count = self.display_orders.len() as i32;
        if columns_count == 0 {
            return;
        }
        let dst_order = dst_order.clamp(0, columns_count - 1);
        if src_order == dst_order {
            return;
        }
        let reorder_dir = if dst_order < src_order { -1 } else { 1 };
        // The columns between the two positions, read from the map before it is rebuilt.
        let mut shifted = Vec::new();
        let mut order_n = src_order + reorder_dir;
        while order_n != dst_order + reorder_dir {
            if let Some(&n) = self.display_order_to_index.get(order_n as usize) {
                shifted.push(n as usize);
            }
            order_n += reorder_dir;
        }
        self.display_orders[column] = dst_order;
        for n in shifted {
            if let Some(entry) = self.display_orders.get_mut(n) {
                *entry -= reorder_dir;
            }
        }
        self.fix_display_order();
    }

    /// `TableGetMaxDisplayOrderAllowed`: clamps the destination of a drag so it cannot cross a column
    /// that refuses reordering. Frozen columns are not modelled.
    fn max_display_order_allowed(&self, src_order: i32, dst_order: i32) -> i32 {
        let columns_count = self.columns.len() as i32;
        if columns_count == 0 {
            return 0;
        }
        let mut dst_order = dst_order.clamp(0, columns_count - 1);
        if src_order == dst_order {
            return dst_order;
        }
        let reorder_dir = if src_order < dst_order { 1 } else { -1 };
        let mut order_n = src_order;
        while (src_order < dst_order && order_n <= dst_order)
            || (dst_order < src_order && order_n >= dst_order)
        {
            if let Some(&n) = self.display_order_to_index.get(order_n as usize) {
                let refuses = self
                    .columns
                    .get(n as usize)
                    .map(|column| column.flags & column_flags::NO_REORDER != 0)
                    .unwrap_or(false);
                if refuses {
                    dst_order = if order_n == src_order {
                        src_order
                    } else {
                        order_n - reorder_dir
                    };
                    break;
                }
            }
            order_n += reorder_dir;
        }
        dst_order
    }

    /// `TableQueueSetColumnDisplayOrder`: a header drag asks for a move, which the next `BeginTable`
    /// applies. `layout_locked` mirrors the reference's early-out for a request that would not move.
    pub fn queue_column_display_order(&mut self, column: usize, dst_order: i32) {
        let Some(&src_order) = self.display_orders.get(column) else {
            return;
        };
        self.reorder_column = column as i32;
        self.reorder_column_dst_order = -1;
        let dst_order = self.max_display_order_allowed(src_order, dst_order);
        if self.layout_locked && dst_order == src_order {
            return;
        }
        self.reorder_column_dst_order = dst_order;
    }

    /// `BeginTableEx`'s "Handle reordering request": applies the move queued by the previous frame.
    pub fn apply_pending_reorder(&mut self) {
        if self.reorder_column != -1 && self.reorder_column_dst_order != -1 {
            let column = self.reorder_column as usize;
            let dst_order = self.reorder_column_dst_order;
            self.set_column_display_order(column, dst_order);
            self.reorder_column_dst_order = -1;
        }
    }

    /// `TableUpdateLayout`, for the paths the first cut supports: every column is `WidthFixed` with
    /// a width of its own, nothing is hidden or stretched, the columns follow their display order,
    /// and the table neither scrolls nor freezes.
    ///
    /// `cell_padding` is `ImGuiStyle::CellPadding` and `host_clip` is the window's clip rectangle,
    /// which the reference keeps as `table->HostClipRect`.
    pub fn update_layout(&mut self, cell_padding: Vec2, host_clip: [f32; 4], frame_padding_x: f32) {
        // The padding and spacing arithmetic of `BeginTableEx`: without vertical borders the space
        // between two columns is the explicit cell padding on both sides; with them, it is the
        // border, and the padding moves inside the column instead.
        let borders_inner_v = self.flags & table_flags::BORDERS_INNER_V != 0;
        let borders_outer_v = self.flags & table_flags::BORDERS_OUTER_V != 0;
        let pad_inner_x = self.flags & table_flags::NO_PAD_INNER_X == 0;
        let inner_spacing_for_border = if borders_inner_v { BORDER_SIZE } else { 0.0 };
        let inner_spacing_explicit = if pad_inner_x && !borders_inner_v {
            cell_padding.x
        } else {
            0.0
        };
        self.cell_spacing_x1 = inner_spacing_explicit + inner_spacing_for_border;
        self.cell_spacing_x2 = inner_spacing_explicit;
        self.cell_padding_x = if pad_inner_x && borders_inner_v {
            cell_padding.x
        } else {
            0.0
        };
        // `PadOuterX` is the default when there are vertical borders, `NoPadOuterX` disables it.
        let pad_outer_x = if self.flags & table_flags::NO_PAD_OUTER_X != 0 {
            false
        } else if self.flags & table_flags::PAD_OUTER_X != 0 {
            true
        } else {
            borders_outer_v
        };
        let outer_padding_for_border = if borders_outer_v { BORDER_SIZE } else { 0.0 };
        let outer_padding_explicit = if pad_outer_x { cell_padding.x } else { 0.0 };
        self.outer_padding_x =
            (outer_padding_for_border + outer_padding_explicit) - self.cell_padding_x;

        // `InnerClipRect`: the table's rectangle shrunk by the outer border size when there is one
        // (a 1 px outer border pulls the clip in on both sides), clipped to the window, then carried
        // down to the window's bottom edge so the rows can grow inside it. The vertical shrink of a
        // table with no height yet collapses to its top, which is why only x moves here.
        let border_outer_v = if borders_outer_v { BORDER_SIZE } else { 0.0 };
        self.inner_clip = (
            Vec2 {
                x: self.outer_rect.0.x.max(host_clip[0]) + border_outer_v,
                y: self.outer_rect.0.y.max(host_clip[1]),
            },
            Vec2 {
                x: self.outer_rect.1.x.min(host_clip[2]) - border_outer_v,
                y: host_clip[3],
            },
        );
        self.border_x1 = self.inner_clip.0.x;
        self.border_x2 = self.inner_clip.1.x;

        // [Part 3] The width each column asks for and, for the stretch ones, its weight. A column
        // declared without a width flag follows the table's sizing policy, which is
        // `SizingFixedFit` when the flags say nothing.
        let policy = self.flags & table_flags::SIZING_MASK;
        let min_column_width = frame_padding_x.max(1.0);
        let resizable = self.flags & table_flags::RESIZABLE != 0;
        // [Part 3a] The width each column would need for its contents, and the sums the stretch
        // policy needs: a column declared without a width flag follows the table policy, which is
        // `SizingFixedFit` when the flags say nothing.
        let mut stretch_sum_width_auto = 0.0f32;
        let mut count_stretch = 0.0f32;
        for column in self.columns.iter_mut() {
            if column.flags & column_flags::WIDTH_MASK == 0 {
                column.flags |= if policy == table_flags::SIZING_FIXED_FIT
                    || policy == table_flags::SIZING_FIXED_SAME
                {
                    column_flags::WIDTH_FIXED
                } else {
                    column_flags::WIDTH_STRETCH
                };
            }
            // `TableGetColumnWidthAuto` from the extents the previous frame left behind. The minimum
            // width is not applied here: on the first frame there is no measurement and the reference
            // still divides by a zero sum, which is what leaves the columns at their minimum.
            column.width_auto = column.content_width;
            if column.flags & column_flags::WIDTH_STRETCH != 0 {
                stretch_sum_width_auto += column.width_auto;
                count_stretch += 1.0;
            }
        }

        // `TableUpdateSpecs` and `TableSortSpecsBuild`: the columns that asked for the default sort
        // become the specs, their sort orders are linearized, and the specs are marked dirty so the
        // caller re-sorts.
        self.build_sort_specs();

        // [Part 3b] The request of each column and, for the stretch ones, its weight.
        let mut sum_width_requests = 0.0f32;
        let mut stretch_sum_weights = 0.0f32;
        for column in self.columns.iter_mut() {
            if column.flags & column_flags::WIDTH_FIXED != 0 {
                // A fixed column that cannot be resized tracks its contents, unless it was declared
                // with an explicit width, which it then keeps.
                if column.init_width > 0.0 && !resizable {
                    column.width_auto = column.init_width;
                }
                if !resizable {
                    column.width_request = column.width_auto.max(min_column_width);
                }
                sum_width_requests += column.width_request;
            } else {
                // `InitStretchWeightOrWidth` wins when the column asked for one; otherwise
                // `SizingStretchProp` weighs the column by the width of its contents, and every
                // other policy gives it an equal share.
                column.stretch_weight = if column.init_width > 0.0 {
                    column.init_width
                } else if policy == table_flags::SIZING_STRETCH_PROP {
                    // With no contents measured yet the sum is 0, so this is a NaN weight: the
                    // reference relies on it (`NaN > 0` is false) to leave the columns at their
                    // minimum width on the frame that has no measurements.
                    (column.width_auto / stretch_sum_width_auto) * count_stretch
                } else {
                    1.0
                };
                stretch_sum_weights += column.stretch_weight;
            }
            sum_width_requests += self.cell_padding_x * 2.0;
        }

        // [Part 4] Final widths: the stretch columns share what the fixed ones leave, once the
        // spacing between columns is taken out.
        let width_avail = (self.outer_rect.1.x - self.outer_rect.0.x).max(1.0);
        let width_spacings = self.outer_padding_x * 2.0
            + (self.cell_spacing_x1 + self.cell_spacing_x2)
                * self.columns.len().saturating_sub(1) as f32;
        let width_avail_for_stretched = width_avail - width_spacings - sum_width_requests;
        let mut width_remaining = width_avail_for_stretched;
        for column in self.columns.iter_mut() {
            if column.flags & column_flags::WIDTH_STRETCH != 0 {
                let ratio = if stretch_sum_weights > 0.0 {
                    column.stretch_weight / stretch_sum_weights
                } else {
                    0.0
                };
                column.width_request =
                    ((width_avail_for_stretched * ratio).max(min_column_width) + 0.01).trunc();
                width_remaining -= column.width_request;
            }
            column.width_given = column.width_request.max(min_column_width).trunc();
        }

        // [Part 5] The rounding remainder goes to the stretch columns, from right to left.
        if width_remaining >= 1.0
            && stretch_sum_weights > 0.0
            && self.flags & table_flags::PRECISE_WIDTHS == 0
        {
            for column in self.columns.iter_mut().rev() {
                if width_remaining < 1.0 {
                    break;
                }
                if column.flags & column_flags::WIDTH_STRETCH == 0 {
                    continue;
                }
                column.width_request += 1.0;
                column.width_given += 1.0;
                width_remaining -= 1.0;
            }
        }

        // The column chain: the first column starts one spacing before the table's content start,
        // each column begins where the previous one ended, and the walk follows the display order the
        // reorder drag rewrites.
        let mut min_x = self.outer_rect.0.x + self.outer_padding_x - self.cell_spacing_x1;
        let display_order = self.display_order_to_index.clone();
        for (order_n, column_n) in display_order.iter().enumerate() {
            let n = *column_n as usize;
            let next_enabled_column = display_order.get(order_n + 1).copied().unwrap_or(-1);
            let sorted = self.sort_orders.get(n).copied().unwrap_or(-1) != -1;
            let Some(column) = self.columns.get_mut(n) else {
                continue;
            };
            // `TableUpdateLayout` clears the status bits and marks the column it lays out.
            column.flags &= !column_flags::STATUS_MASK;
            column.flags |= column_flags::IS_ENABLED | column_flags::IS_VISIBLE;
            if sorted {
                column.flags |= column_flags::IS_SORTED;
            }
            column.next_enabled_column = next_enabled_column;
            column.min_x = min_x;
            column.work_min_x = min_x + self.cell_spacing_x1 + self.cell_padding_x;
            column.max_x =
                column.work_min_x + column.width_given + self.cell_padding_x + self.cell_spacing_x2;
            column.work_max_x = column.max_x - self.cell_padding_x - self.cell_spacing_x2;
            column.clip = [
                column.min_x.max(self.inner_clip.0.x),
                self.outer_rect.0.y,
                column.max_x.min(self.inner_clip.1.x),
                self.inner_clip.1.y,
            ];
            // The content extent restarts at the column's work edge on every layout.
            column.content_max_x = column.work_min_x;
            min_x = column.max_x;
        }
        // The columns keep their declared index as their identity; only the chain above moves.

        self.work_rect = self.outer_rect;
        self.bg_clip = self.inner_clip;
        self.row_cell_padding_y = cell_padding.y;
        self.row_pos_y1 = self.outer_rect.0.y;
        self.row_pos_y2 = self.outer_rect.0.y;
        self.layout_locked = true;
    }

    /// `TableBeginRow` plus the height part of `TableNextRow`: the row starts where the previous one
    /// ended and reserves its vertical padding up front.
    pub fn begin_row(&mut self, row_min_height: f32, row_flags: u32, row_bg: Option<u32>) {
        self.current_row += 1;
        self.current_column = -1;
        self.is_inside_row = true;
        self.last_row_flags = self.row_flags;
        self.row_flags = row_flags;
        self.row_min_height = row_min_height;
        self.row_bg0 = row_bg;
        self.row_cells.clear();
        // `TableEndRow` only counts the rows that are not headers, which is what makes the bands of
        // `RowBg` alternate over the data rows while the header keeps its own colour.
        if row_flags & table_flags::ROW_HEADERS == 0 {
            self.row_bg_counter += 1;
        }
        self.row_pos_y1 = self.row_pos_y2;
        self.row_pos_y2 += self.row_cell_padding_y * 2.0;
        self.row_pos_y2 = self.row_pos_y2.max(self.row_pos_y1 + row_min_height);
    }

    /// The `TableEndCell` part that grows the row: the cell's content plus its vertical padding.
    pub fn end_cell(&mut self, cursor_max_y: f32) {
        self.row_pos_y2 = self.row_pos_y2.max(cursor_max_y + self.row_cell_padding_y);
    }

    /// `TableMergeDrawChannels`' test, which only looks at the right edge: a column whose content
    /// stayed inside its clip can share a command with the others.
    pub fn column_content_fits(&self, column: usize) -> bool {
        self.columns[column].content_max_x <= self.columns[column].clip[2]
    }

    /// `TableGetCellBgRect`: the column's span clipped to the work rectangle, over the current row.
    pub fn cell_rect(&self, column: usize) -> (Vec2, Vec2) {
        let column = &self.columns[column];
        (
            Vec2 {
                x: column.min_x.max(self.work_rect.0.x),
                y: self.row_pos_y1,
            },
            Vec2 {
                x: column.max_x.min(self.work_rect.1.x),
                y: self.row_pos_y2,
            },
        )
    }

    /// `TableSetBgColor(ImGuiTableBgTarget_CellBg, ...)`: the background of the current cell, which
    /// is painted when the row closes. The reference reuses the last entry when the column did not
    /// change.
    pub fn set_cell_bg(&mut self, color: u32) {
        if self.current_column < 0 {
            return;
        }
        let column = self.current_column as usize;
        if let Some(last) = self.row_cells.last_mut() {
            if last.0 == column {
                last.1 = color;
                return;
            }
        }
        self.row_cells.push((column, color));
    }

    /// `TableGetCellBgRect` clipped the way `TableEndRow` paints it: first to the table's background
    /// clip, then to the column's own span.
    pub fn cell_bg_rect(&self, column: usize) -> (Vec2, Vec2) {
        let (mut min, mut max) = self.cell_rect(column);
        min.x = min.x.max(self.bg_clip.0.x);
        min.y = min.y.max(self.bg_clip.0.y);
        max.x = max.x.min(self.bg_clip.1.x);
        max.y = max.y.min(self.bg_clip.1.y);
        // `TableEndRow` then keeps the fill inside the column's clip and its `MaxX`.
        min.x = min.x.max(self.columns[column].clip[0]);
        max.x = max.x.min(self.columns[column].max_x);
        (min, max)
    }

    /// The top of the content of the cell in `column`, which is where its first item goes.
    pub fn cell_min(&self, column: usize) -> Vec2 {
        Vec2 {
            x: self.columns[column].work_min_x,
            y: self.row_pos_y1 + self.row_cell_padding_y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Table {
        let mut table = Table::new(
            0,
            2,
            (Vec2 { x: 68.0, y: 68.0 }, Vec2 { x: 352.0, y: 68.0 }),
        );
        table.setup_column("A", column_flags::WIDTH_FIXED, 60.0);
        table.setup_column("B", column_flags::WIDTH_FIXED, 80.0);
        table.update_layout(Vec2 { x: 4.0, y: 2.0 }, [61.0, 61.0, 359.0, 259.0], 4.0);
        table
    }

    /// The numbers the reference printed for columns of 60 and 80 px in a 300x200 window at (60,60)
    /// with no borders: content at x 68 and 136, each column 68 px wide from its padding edge.
    #[test]
    fn fixed_columns_match_the_reference_geometry() {
        let table = table();
        assert_eq!(table.columns[0].min_x, 64.0);
        assert_eq!(table.columns[0].work_min_x, 68.0);
        assert_eq!(table.columns[0].max_x, 132.0);
        assert_eq!(table.columns[0].work_max_x, 128.0);
        assert_eq!(table.columns[1].min_x, 132.0);
        assert_eq!(table.columns[1].work_min_x, 136.0);
        assert_eq!(table.columns[1].max_x, 220.0);
        assert_eq!(table.inner_clip.1, Vec2 { x: 352.0, y: 259.0 });
    }

    /// `CellPadding.y` is 2 by default, so a row holding 13 px of text is 17 px tall.
    #[test]
    fn a_row_grows_to_its_content_plus_padding() {
        let mut table = table();
        table.begin_row(0.0, 0, None);
        assert_eq!(table.row_pos_y1, 68.0);
        assert_eq!(table.row_pos_y2, 72.0);
        table.end_cell(83.0);
        assert_eq!(table.row_pos_y2, 85.0);
    }

    /// A table with the columns `A` (default sort), `B` (default sort, descending) and `C` (no sort),
    /// as the `tables_sort_arrow` and `tables_sort_click` goldens declare it.
    fn sortable_table(flags: u32) -> Table {
        let mut table = Table::new(
            flags,
            3,
            (Vec2 { x: 68.0, y: 68.0 }, Vec2 { x: 352.0, y: 68.0 }),
        );
        table.setup_column(
            "A",
            column_flags::WIDTH_FIXED | column_flags::DEFAULT_SORT,
            60.0,
        );
        table.setup_column(
            "B",
            column_flags::WIDTH_FIXED
                | column_flags::DEFAULT_SORT
                | column_flags::PREFER_SORT_DESCENDING,
            80.0,
        );
        table.setup_column(
            "C",
            column_flags::WIDTH_FIXED | column_flags::NO_SORT,
            100.0,
        );
        // `BeginTableEx` marks the specs dirty on the frame that creates the table.
        table.sort_specs_dirty = true;
        table
    }

    /// Two `DefaultSort` columns both start at sort order 0; the sanitize step is what linearizes
    /// them to 0 and 1, and only `SortMulti` lets both keep one.
    #[test]
    fn default_sorts_are_linearized_and_multi_keeps_them() {
        let mut table = sortable_table(table_flags::SORTABLE | table_flags::SORT_MULTI);
        table.build_sort_specs();
        assert_eq!(table.sort_specs, vec![(0, 1), (1, 2)]);
        assert!(table.sort_specs_dirty);
        assert!(table.sort_specs_public_dirty);

        let mut single = sortable_table(table_flags::SORTABLE);
        single.build_sort_specs();
        assert_eq!(single.sort_specs, vec![(0, 1)]);
    }

    /// A sortable table without a `DefaultSort` column starts with its first sortable column, which
    /// is the rule the `tables_sort_tristate` golden pins (the reference run that produced it had a
    /// sort saved in its settings, so the port always hands out one spec).
    #[test]
    fn a_sortable_table_without_defaults_starts_on_its_first_sortable_column() {
        let mut table = Table::new(
            table_flags::SORTABLE | table_flags::SORT_TRISTATE,
            2,
            (Vec2 { x: 68.0, y: 68.0 }, Vec2 { x: 352.0, y: 68.0 }),
        );
        table.setup_column("A", column_flags::WIDTH_FIXED, 60.0);
        table.setup_column("B", column_flags::WIDTH_FIXED | column_flags::NO_SORT, 80.0);
        table.build_sort_specs();
        assert_eq!(table.sort_specs, vec![(0, 1)]);
    }

    /// The direction a click would set: unsorted columns take the first available direction, sorted
    /// ones take the next in the list, and `NoSortAscending` removes ascending from the list.
    #[test]
    fn the_next_direction_follows_the_available_list() {
        // `A` starts sorted ascending by its `DefaultSort` flag: the next click turns it descending,
        // and the one after it back to ascending.
        let mut table = sortable_table(table_flags::SORTABLE);
        assert_eq!(table.next_sort_direction(0), 2);
        table.set_column_sort_direction(0, 2, false);
        assert_eq!(table.next_sort_direction(0), 1);

        // An unsorted column starts at the first available direction.
        let mut plain = Table::new(
            table_flags::SORTABLE,
            1,
            (Vec2 { x: 68.0, y: 68.0 }, Vec2 { x: 352.0, y: 68.0 }),
        );
        plain.setup_column("A", column_flags::WIDTH_FIXED, 60.0);
        assert_eq!(plain.next_sort_direction(0), 1);
        // With ascending banned the list loses it, so the same column starts descending.
        plain.columns[0].flags |= column_flags::NO_SORT_ASCENDING;
        assert_eq!(plain.next_sort_direction(0), 2);
    }

    /// Without `SortMulti` a click replaces the specs; with it and `append` (the Shift path) the
    /// column goes to `max(order) + 1` and the others keep theirs.
    #[test]
    fn a_click_replaces_or_appends_the_order() {
        let mut table = sortable_table(table_flags::SORTABLE | table_flags::SORT_MULTI);
        table.build_sort_specs();
        assert_eq!(table.sort_specs, vec![(0, 1), (1, 2)]);

        table.set_column_sort_direction(2, 1, true);
        assert_eq!(table.sort_order(2), 2);
        table.build_sort_specs();
        assert_eq!(table.sort_specs, vec![(0, 1), (1, 2), (2, 1)]);

        // The same call without `append` replaces the whole order with this column at 0.
        table.set_column_sort_direction(2, 2, false);
        table.build_sort_specs();
        assert_eq!(table.sort_specs, vec![(2, 2)]);
        assert_eq!(table.sort_order(0), -1);
        assert_eq!(table.sort_order(1), -1);

        // `SortMulti` without Shift cannot append either.
        let mut plain = sortable_table(table_flags::SORTABLE);
        plain.build_sort_specs();
        plain.set_column_sort_direction(1, 1, true);
        plain.build_sort_specs();
        assert_eq!(plain.sort_specs, vec![(1, 1)]);
        assert_eq!(plain.sort_order(0), -1);
    }

    /// `TableSetColumnDisplayOrder` shifts the columns between the two positions, and `DisplayOrder`
    /// and `DisplayOrderToIndex` stay each other's inverse.
    #[test]
    fn moving_a_column_shifts_the_ones_in_between() {
        let mut table = sortable_table(0);
        assert_eq!(table.display_orders, vec![0, 1, 2]);
        table.set_column_display_order(1, 0);
        assert_eq!(table.display_orders, vec![1, 0, 2]);
        assert_eq!(table.display_order_to_index, vec![1, 0, 2]);
        table.set_column_display_order(2, 0);
        assert_eq!(table.display_orders, vec![2, 1, 0]);
        assert_eq!(table.display_order_to_index, vec![2, 1, 0]);
        assert_eq!(table.column_at_display_order(2), 0);
    }

    /// A drag cannot cross a column that refuses reordering, which is the only barrier modelled until
    /// frozen columns land.
    #[test]
    fn the_drag_cannot_cross_a_no_reorder_column() {
        let mut table = sortable_table(0);
        table.columns[0].flags |= column_flags::NO_REORDER;
        table.queue_column_display_order(1, 0);
        assert_eq!(table.reorder_column_dst_order, 1);
        table.apply_pending_reorder();
        assert_eq!(table.display_orders, vec![0, 1, 2]);
    }

    /// The sort state that outlives the frame round-trips, and a restored table ignores the
    /// `DefaultSort` flags of the columns declared again.
    #[test]
    fn the_saved_state_round_trips_and_overrides_the_defaults() {
        let mut table = sortable_table(table_flags::SORTABLE | table_flags::SORT_MULTI);
        table.build_sort_specs();
        table.set_column_sort_direction(0, 0, false);
        assert_eq!(table.sort_orders, vec![-1, -1, -1]);
        let state = table.sort_state();
        assert!(state.specs_dirty);

        let mut restored = sortable_table(table_flags::SORTABLE | table_flags::SORT_MULTI);
        assert!(restored.restore_sort_state(&state));
        assert!(restored.sort_state_loaded);
        // Declaring `A` again with `DefaultSort` must not put its order back to 0.
        restored.setup_column(
            "A",
            column_flags::WIDTH_FIXED | column_flags::DEFAULT_SORT,
            60.0,
        );
        assert_eq!(restored.sort_orders[0], -1);
    }
}
