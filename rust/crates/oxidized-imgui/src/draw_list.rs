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
}

impl DrawList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.vtx_buffer.clear();
        self.idx_buffer.clear();
        self.cmd_buffer.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.vtx_buffer.is_empty() && self.idx_buffer.is_empty()
    }

    /// Appends a filled axis-aligned rectangle.
    ///
    /// Provisional geometry: four vertices, six indices, one command.
    pub fn add_rect_filled(&mut self, min: Vec2, max: Vec2, col: ImU32) {
        let base = self.vtx_buffer.len() as u32;
        let uv = Vec2::default();
        for (x, y) in [
            (min.x, min.y),
            (max.x, min.y),
            (max.x, max.y),
            (min.x, max.y),
        ] {
            self.vtx_buffer.push(DrawVert {
                pos: Vec2 { x, y },
                uv,
                col,
            });
        }
        for i in [0u16, 1, 2, 0, 2, 3] {
            self.idx_buffer.push(base as ImDrawIdx + i);
        }
        self.cmd_buffer.push(DrawCmd {
            elem_count: 6,
            ..DrawCmd::default()
        });
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
        assert!(list.cmd_buffer.is_empty());
    }
}
