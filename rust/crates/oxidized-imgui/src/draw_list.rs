//! Draw data structures.
//!
//! The field order, types and padding mirror `ImDrawVert`/`ImDrawCmd` in `imgui.h` 1.93.1 so the
//! buffers have the same byte layout a backend expects. The tests pin the sizes and offsets the
//! C++ reference reports (`rust/tools/oracle`).
//!
//! Geometry generation (the `add_*` helpers) is provisional: it produces valid buffers, but
//! matching the reference's vertices and indices for the same calls is the M1 exit test and is not
//! claimed yet.

use std::ffi::c_void;

/// 2D vector, `ImVec2`.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

/// A packed RGBA color, `ImU32`.
pub type ImU32 = u32;

/// Vertex index, `ImDrawIdx` (16-bit by default upstream).
pub type ImDrawIdx = u16;

/// `ImDrawVert`: position, texture coordinates, packed color.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct DrawVert {
    pub pos: Vec2,
    pub uv: Vec2,
    pub col: ImU32,
}

/// Texture reference, `ImTextureRef` (16 bytes: data pointer plus user texture id).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TexRef {
    pub tex_data: *mut c_void,
    pub tex_id: u64,
}

impl Default for TexRef {
    fn default() -> Self {
        Self {
            tex_data: core::ptr::null_mut(),
            tex_id: 0,
        }
    }
}

/// `ImDrawCallback`.
pub type DrawCallback = unsafe extern "C" fn(parent_list: *const DrawList, cmd: *const DrawCmd);

/// `ImDrawCmd`: one draw call over a slice of the vertex and index buffers.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DrawCmd {
    pub clip_rect: [f32; 4],
    pub tex_ref: TexRef,
    pub vtx_offset: u32,
    pub idx_offset: u32,
    pub elem_count: u32,
    pub user_callback: Option<DrawCallback>,
    pub user_callback_data: *mut c_void,
    pub user_callback_data_size: i32,
    pub user_callback_data_offset: i32,
}

impl Default for DrawCmd {
    fn default() -> Self {
        Self {
            clip_rect: [0.0; 4],
            tex_ref: TexRef::default(),
            vtx_offset: 0,
            idx_offset: 0,
            elem_count: 0,
            user_callback: None,
            user_callback_data: core::ptr::null_mut(),
            user_callback_data_size: 0,
            user_callback_data_offset: -1,
        }
    }
}

/// A vertex and index buffer with the draw commands that walk it.
#[derive(Clone, Debug, Default)]
pub struct DrawList {
    pub vtx_buffer: Vec<DrawVert>,
    pub idx_buffer: Vec<ImDrawIdx>,
    pub cmd_buffer: Vec<DrawCmd>,
    pub flags: u32,
    /// `ImDrawListSharedData::TexUvWhitePixel`: the UV a solid fill samples. The reference takes it
    /// from the font atlas; here it is an explicit input so the harness can pin it.
    pub white_uv: Vec2,
    /// Clip used when the clip stack is empty (`ImDrawListSharedData::ClipRectFullscreen`).
    pub fullscreen_clip: [f32; 4],
    clip_stack: Vec<[f32; 4]>,
    current_clip: [f32; 4],
}

impl DrawList {
    /// Resets the list for a new frame: buffers empty and one command ready, like the reference's
    /// `_ResetForNewFrame` ("We always have a command ready in the buffer").
    pub fn new() -> Self {
        Self {
            cmd_buffer: vec![DrawCmd::default()],
            fullscreen_clip: [0.0, 0.0, 1280.0, 720.0],
            ..Self::default()
        }
    }

    /// `ImDrawList::PushClipRect`: intersects with the current clip when asked, then starts a new
    /// command if the current one already has geometry.
    pub fn push_clip_rect(&mut self, min: Vec2, max: Vec2, intersect_with_current: bool) {
        let mut cr = [min.x, min.y, max.x, max.y];
        if intersect_with_current {
            let current = self.current_clip;
            cr[0] = cr[0].max(current[0]);
            cr[1] = cr[1].max(current[1]);
            cr[2] = cr[2].min(current[2]);
            cr[3] = cr[3].min(current[3]);
        }
        cr[2] = cr[2].max(cr[0]);
        cr[3] = cr[3].max(cr[1]);
        self.clip_stack.push(cr);
        self.current_clip = cr;
        self.on_changed_clip_rect();
    }

    /// `ImDrawList::PopClipRect`: back to the enclosing clip, or the fullscreen one if none.
    pub fn pop_clip_rect(&mut self) {
        self.clip_stack.pop();
        self.current_clip = self
            .clip_stack
            .last()
            .copied()
            .unwrap_or(self.fullscreen_clip);
        self.on_changed_clip_rect();
    }

    /// `ImDrawList::_OnChangedClipRect`.
    fn on_changed_clip_rect(&mut self) {
        let last_has_geometry = self
            .cmd_buffer
            .last()
            .map(|cmd| cmd.elem_count > 0)
            .unwrap_or(false);
        if last_has_geometry {
            let cmd = DrawCmd {
                clip_rect: self.current_clip,
                ..DrawCmd::default()
            };
            self.cmd_buffer.push(cmd);
        } else if let Some(cmd) = self.cmd_buffer.last_mut() {
            cmd.clip_rect = self.current_clip;
        }
    }

    /// Resets the buffers, keeping the list reusable.
    pub fn reset_for_new_frame(&mut self) {
        *self = Self::new();
    }

    /// Clears every buffer and leaves the list ready for a new frame, as `ImDrawList::Clear`.
    pub fn clear(&mut self) {
        self.reset_for_new_frame();
    }

    pub fn is_empty(&self) -> bool {
        self.vtx_buffer.is_empty() && self.idx_buffer.is_empty()
    }

    /// `ImDrawList::PrimRectUV`: vertex order a, b=(c.x,a.y), c, d=(a.x,c.y); indices
    /// 0-1-2, 0-2-3; six indices added to the current command.
    pub fn prim_rect_uv(&mut self, a: Vec2, c: Vec2, uv_a: Vec2, uv_c: Vec2, col: ImU32) {
        let b = Vec2 { x: c.x, y: a.y };
        let d = Vec2 { x: a.x, y: c.y };
        let uv_b = Vec2 {
            x: uv_c.x,
            y: uv_a.y,
        };
        let uv_d = Vec2 {
            x: uv_a.x,
            y: uv_c.y,
        };

        let base = self.vtx_buffer.len() as u32;
        for (pos, uv) in [(a, uv_a), (b, uv_b), (c, uv_c), (d, uv_d)] {
            self.vtx_buffer.push(DrawVert { pos, uv, col });
        }
        for i in [0u32, 1, 2, 0, 2, 3] {
            self.idx_buffer.push((base + i) as ImDrawIdx);
        }
        if let Some(cmd) = self.cmd_buffer.last_mut() {
            cmd.elem_count += 6;
        }
    }

    /// `ImDrawList::PrimRect`: a rectangle filled with the white pixel, no explicit UVs.
    pub fn prim_rect(&mut self, a: Vec2, c: Vec2, col: ImU32) {
        let uv = self.white_uv;
        self.prim_rect_uv(a, c, uv, uv, col);
    }

    /// Appends a filled axis-aligned rectangle.
    ///
    /// Only the pixel-aligned path is implemented: the reference reduces `AddRectFilled` to
    /// `PrimRect` when rounding is zero and the coordinates need no anti-aliasing. Rounded,
    /// anti-aliased and sub-pixel rectangles are not implemented yet.
    pub fn add_rect_filled(&mut self, min: Vec2, max: Vec2, col: ImU32) {
        self.prim_rect(min, max, col);
    }

    /// `AddLineH` with the default inside stroke: the reference's pixel-aligned fast path, which is
    /// a `PrimRect` from `(min_x, y)` to `(max_x, y + thickness)`.
    pub fn add_line_h(&mut self, min_x: f32, max_x: f32, y: f32, col: ImU32, thickness: f32) {
        self.prim_rect(
            Vec2 { x: min_x, y },
            Vec2 {
                x: max_x,
                y: y + thickness,
            },
            col,
        );
    }

    pub fn vertices(&self) -> &[DrawVert] {
        &self.vtx_buffer
    }

    pub fn indices(&self) -> &[ImDrawIdx] {
        &self.idx_buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    // Values printed by the C++ reference (`rust/tools/oracle`), x86_64 MSVC.
    #[test]
    fn layouts_match_reference() {
        assert_eq!(size_of::<Vec2>(), 8);
        assert_eq!(size_of::<DrawVert>(), 20);
        assert_eq!(offset_of!(DrawVert, pos), 0);
        assert_eq!(offset_of!(DrawVert, uv), 8);
        assert_eq!(offset_of!(DrawVert, col), 16);

        assert_eq!(size_of::<DrawCmd>(), 72);
        assert_eq!(offset_of!(DrawCmd, clip_rect), 0);
        assert_eq!(offset_of!(DrawCmd, tex_ref), 16);
        assert_eq!(offset_of!(DrawCmd, vtx_offset), 32);
        assert_eq!(offset_of!(DrawCmd, idx_offset), 36);
        assert_eq!(offset_of!(DrawCmd, elem_count), 40);
        assert_eq!(offset_of!(DrawCmd, user_callback), 48);

        assert_eq!(size_of::<TexRef>(), 16);
        assert_eq!(size_of::<ImDrawIdx>(), 2);
    }

    #[test]
    fn rect_is_four_vertices_six_indices() {
        let mut list = DrawList::new();
        list.add_rect_filled(
            Vec2 { x: 10.0, y: 20.0 },
            Vec2 { x: 30.0, y: 40.0 },
            0xFF00_00FF,
        );
        assert_eq!(list.vertices().len(), 4);
        assert_eq!(list.indices().len(), 6);
        assert_eq!(list.cmd_buffer.len(), 1);
        assert_eq!(list.cmd_buffer[0].elem_count, 6);
        assert_eq!(list.vertices()[0].pos, Vec2 { x: 10.0, y: 20.0 });
        assert_eq!(list.vertices()[2].pos, Vec2 { x: 30.0, y: 40.0 });
        assert!(list.indices().iter().all(|&i| (i as usize) < 4));

        list.clear();
        assert!(list.is_empty());
        assert_eq!(list.cmd_buffer.len(), 1);
        assert_eq!(list.cmd_buffer[0].elem_count, 0);
    }

    #[test]
    fn new_frame_leaves_one_command_ready() {
        let list = DrawList::new();
        assert!(list.is_empty());
        assert_eq!(list.cmd_buffer.len(), 1);
        assert_eq!(list.cmd_buffer[0].elem_count, 0);
    }
}
