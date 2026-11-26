//! `ImGuiTable`: the layout of a table with fixed columns.
//!
//! Only the paths the `tables_basic` golden pins are here: columns sized with `WidthFixed`, rows
//! that grow to their contents, and the cell padding and spacing the reference derives from
//! `ImGuiStyle::CellPadding`. Headers, the borders themselves, sizing policies, reordering, freezing
//! and the table's own scrolling are still missing; the geometry they add is measured in
//! `docs/TRASPASO.md`.

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
    /// `ImGuiTableFlags_RowBg`: every other row gets `TableRowBgAlt`.
    pub const ROW_BG: u32 = 1 << 6;
    /// `ImGuiTableFlags_SizingMask_`: the bits that pick the table sizing policy.
    pub const SIZING_MASK: u32 = 4 << 13;
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
    /// `ImGuiTableColumnFlags_NoResize`.
    pub const NO_RESIZE: u32 = 1 << 5;
}

/// `TABLE_BORDER_SIZE`.
const BORDER_SIZE: f32 = 1.0;

/// One column, `ImGuiTableColumn`, with the geometry `TableUpdateLayout` resolves for it.
#[derive(Clone, Debug, Default)]
pub struct TableColumn {
    /// `column->NameOffset` resolved: the label `TableSetupColumn` was given.
    pub name: String,
    pub flags: u32,
    /// `column->WidthRequest`: what `TableSetupColumn` asked for.
    pub width_request: f32,
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
            ..Self::default()
        }
    }

    /// `ImGui::TableSetupColumn`: the columns are declared in order, before the first row.
    pub fn setup_column(&mut self, label: &str, flags: u32, width: f32) {
        let index = self.decl_columns;
        if index >= self.columns.len() {
            return;
        }
        self.columns[index] = TableColumn {
            name: label.to_string(),
            flags,
            width_request: width,
            ..TableColumn::default()
        };
        self.decl_columns += 1;
    }

    /// `TableUpdateLayout`, for the paths the first cut supports: every column is `WidthFixed` with
    /// a width of its own, nothing is hidden, stretched or reordered, and the table neither scrolls
    /// nor freezes.
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
        let mut sum_width_requests = 0.0f32;
        let mut stretch_sum_weights = 0.0f32;
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
            if column.flags & column_flags::WIDTH_FIXED != 0 {
                sum_width_requests += column.width_request;
            } else {
                // `InitStretchWeightOrWidth` is the weight when the column asked for one; the
                // content-derived weight of `SizingStretchProp` is not ported yet.
                column.stretch_weight = if column.width_request > 0.0 {
                    column.width_request
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
        if width_remaining >= 1.0 && self.flags & table_flags::PRECISE_WIDTHS == 0 {
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
        // and each column begins where the previous one ended.
        let mut min_x = self.outer_rect.0.x + self.outer_padding_x - self.cell_spacing_x1;
        let columns_count = self.columns.len() as i32;
        for (n, column) in self.columns.iter_mut().enumerate() {
            column.next_enabled_column = if (n as i32) + 1 < columns_count {
                n as i32 + 1
            } else {
                -1
            };
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
}
