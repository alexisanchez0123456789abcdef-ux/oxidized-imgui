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
    /// `_FringeScale`: 1 / pixel density. Drives the width of the anti-aliased fringe.
    pub fringe_scale: f32,
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

    /// `ImDrawList::PrimQuadUV`: four vertices with explicit UVs and six indices.
    // One argument per corner per UV: the reference's signature, kept on purpose.
    #[allow(clippy::too_many_arguments)]
    pub fn prim_quad_uv(
        &mut self,
        a: Vec2,
        b: Vec2,
        c: Vec2,
        d: Vec2,
        uv_a: Vec2,
        uv_b: Vec2,
        uv_c: Vec2,
        uv_d: Vec2,
        col: ImU32,
    ) {
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

    /// `ImFont::RenderText` for a single line without wrapping or fine clipping: one quad per
    /// visible glyph, positioned by the glyph offsets and advanced by `AdvanceX`.
    ///
    /// The glyph data is pinned in [`crate::font::Font::reference_default`]; the atlas is not
    /// built yet.
    pub fn add_text(
        &mut self,
        font: &crate::font::Font,
        size: f32,
        pos: Vec2,
        col: ImU32,
        text: &str,
    ) {
        let scale = size / font.size;
        let mut pen_x = pos.x;
        for ch in text.chars() {
            let Some(glyph) = font.glyph(ch) else {
                continue;
            };
            let x0 = pen_x + glyph.offset[0] * scale;
            let y0 = pos.y + glyph.offset[1] * scale;
            let x1 = pen_x + glyph.offset[2] * scale;
            let y1 = pos.y + glyph.offset[3] * scale;
            if x1 > x0 && y1 > y0 {
                self.prim_quad_uv(
                    Vec2 { x: x0, y: y0 },
                    Vec2 { x: x1, y: y0 },
                    Vec2 { x: x1, y: y1 },
                    Vec2 { x: x0, y: y1 },
                    Vec2 {
                        x: glyph.uv[0],
                        y: glyph.uv[1],
                    },
                    Vec2 {
                        x: glyph.uv[2],
                        y: glyph.uv[1],
                    },
                    Vec2 {
                        x: glyph.uv[2],
                        y: glyph.uv[3],
                    },
                    Vec2 {
                        x: glyph.uv[0],
                        y: glyph.uv[3],
                    },
                    col,
                );
            }
            pen_x += glyph.advance_x * scale;
        }
    }
    /// `ImDrawList::AddConvexPolyFilled` with anti-aliasing, miter joins only (the branch a
    /// rectangle takes). Returns the index count emitted.
    ///
    /// Layout matches the reference: the inner ring first, then one outer vertex per edge, with the
    /// first edge's two triangles patched to wrap onto the last outer vertex.
    pub fn add_convex_poly_filled_aa(&mut self, points: &[Vec2], col: ImU32) -> u32 {
        const MITER_ANGLE_LIMIT: f32 = -0.999_961_9;
        const MITER_LIMIT_SQR: f32 = 4.0 * 4.0;
        const MAX_BEVELS: i32 = 2;
        const A_MASK: u32 = 0xFF00_0000;

        let n = points.len();
        if n < 3 || (col & A_MASK) == 0 {
            return 0;
        }

        let half_aa = self.fringe_scale * 0.5;
        let col_trans = col & !A_MASK;

        // Perimeter normals and the squared length of each incoming edge.
        let mut normals = vec![Vec2 { x: 0.0, y: 0.0 }; n];
        let mut lengths = vec![0.0f32; n];
        for i1 in 0..n {
            let i0 = (i1 + n - 1) % n;
            let (p0, p1) = (points[i0], points[i1]);
            let mut dx = p1.x - p0.x;
            let mut dy = p1.y - p0.y;
            let d2 = dx * dx + dy * dy;
            if d2 > 0.0 {
                let inv = rsqrt_precise(d2);
                dx *= inv;
                dy *= inv;
            }
            normals[i0] = Vec2 { x: dy, y: -dx };
            lengths[i0] = d2;
        }

        let inner_base = self.vtx_buffer.len() as u32;
        self.vtx_buffer.resize(
            self.vtx_buffer.len() + n,
            DrawVert {
                pos: Vec2 { x: 0.0, y: 0.0 },
                uv: self.white_uv,
                col,
            },
        );

        // Per edge: where its outer vertex starts and whether a bevel added a second one.
        let mut outer_start = vec![0u32; n];
        let mut beveled = vec![false; n];
        let mut bevel_count = 0i32;

        for i1 in 0..n {
            let i0 = (i1 + n - 1) % n;
            let n0 = normals[i0];
            let n1 = normals[i1];
            let p1 = points[i1];

            let cos_theta = n0.x * n1.x + n0.y * n1.y;
            let cos_clamped = cos_theta.max(MITER_ANGLE_LIMIT);
            let miter_scale = (1.0 / (1.0 + cos_clamped)).min(1000.0);
            let mut offset_x = (n0.x + n1.x) * miter_scale;
            let mut offset_y = (n0.y + n1.y) * miter_scale;

            let miter_distance_sqr = offset_x * offset_x + offset_y * offset_y;
            let bevel = miter_distance_sqr > MITER_LIMIT_SQR;
            if bevel {
                // Keep the worst-case overshoot no bigger than the polygon itself.
                let reference = half_aa * half_aa;
                let limit_sqr = lengths[i0].max(lengths[i1]);
                let reference_dist_sqr = miter_distance_sqr * reference;
                if reference_dist_sqr > limit_sqr {
                    let scale = (limit_sqr / reference_dist_sqr).sqrt();
                    offset_x *= scale;
                    offset_y *= scale;
                }
            }
            offset_x *= half_aa;
            offset_y *= half_aa;

            self.vtx_buffer[inner_base as usize + i1] = DrawVert {
                pos: Vec2 {
                    x: p1.x - offset_x,
                    y: p1.y - offset_y,
                },
                uv: self.white_uv,
                col,
            };

            outer_start[i1] = self.vtx_buffer.len() as u32;
            if bevel && bevel_count < MAX_BEVELS {
                bevel_count += 1;
                beveled[i1] = true;
                self.vtx_buffer.push(DrawVert {
                    pos: Vec2 {
                        x: p1.x + n0.x * half_aa,
                        y: p1.y + n0.y * half_aa,
                    },
                    uv: self.white_uv,
                    col: col_trans,
                });
                self.vtx_buffer.push(DrawVert {
                    pos: Vec2 {
                        x: p1.x + n1.x * half_aa,
                        y: p1.y + n1.y * half_aa,
                    },
                    uv: self.white_uv,
                    col: col_trans,
                });
            } else {
                self.vtx_buffer.push(DrawVert {
                    pos: Vec2 {
                        x: p1.x + offset_x,
                        y: p1.y + offset_y,
                    },
                    uv: self.white_uv,
                    col: col_trans,
                });
            }
        }

        let last_outer = |i: usize| outer_start[i] + u32::from(beveled[i]);

        let mut idx_count = 0u32;
        for i1 in 0..n {
            let i0 = (i1 + n - 1) % n;
            let prev_outer = if i1 == 0 {
                last_outer(n - 1)
            } else {
                last_outer(i1 - 1)
            };
            let outer = outer_start[i1];
            let inner = inner_base + i1 as u32;
            let prev_inner = inner_base + i0 as u32;
            for vertex in [prev_outer, outer, inner] {
                self.idx_buffer.push(vertex as ImDrawIdx);
            }
            for vertex in [prev_outer, inner, prev_inner] {
                self.idx_buffer.push(vertex as ImDrawIdx);
            }
            idx_count += 6;
            if beveled[i1] {
                for vertex in [outer, outer + 1, inner] {
                    self.idx_buffer.push(vertex as ImDrawIdx);
                }
                idx_count += 3;
            }
        }
        for i in 2..n {
            for vertex in [inner_base, inner_base + i as u32 - 1, inner_base + i as u32] {
                self.idx_buffer.push(vertex as ImDrawIdx);
            }
            idx_count += 3;
        }

        if let Some(cmd) = self.cmd_buffer.last_mut() {
            cmd.elem_count += idx_count;
        }
        idx_count
    }

    /// `ImDrawList::AddRectFilled` for a rectangle that is not pixel aligned, which is the case
    /// where the reference takes its anti-aliased convex-polygon path.
    pub fn add_rect_filled_aa(&mut self, min: Vec2, max: Vec2, col: ImU32) {
        let points = [
            min,
            Vec2 { x: max.x, y: min.y },
            max,
            Vec2 { x: min.x, y: max.y },
        ];
        self.add_convex_poly_filled_aa(&points, col);
    }

    /// `ImDrawList::AddRectFilled` with rounding, antialiased: the corner arcs, then the same
    /// convex-polygon fill.
    ///
    /// The corner sampling step comes from the reference's automatic segment count.
    pub fn add_rect_filled_aa_rounded(&mut self, min: Vec2, max: Vec2, rounding: f32, col: ImU32) {
        let mut rounding = rounding;
        rounding = rounding.min((max.x - min.x).abs() * 0.5 - 1.0);
        rounding = rounding.min((max.y - min.y).abs() * 0.5 - 1.0);
        if rounding < 0.5 {
            self.add_rect_filled_aa(min, max, col);
            return;
        }
        let arc_step = arc_step_for_radius(rounding);

        let mut points: Vec<Vec2> = Vec::new();
        push_arc(
            &mut points,
            Vec2 {
                x: min.x + rounding,
                y: min.y + rounding,
            },
            rounding,
            6,
            9,
            arc_step,
        );
        push_arc(
            &mut points,
            Vec2 {
                x: max.x - rounding,
                y: min.y + rounding,
            },
            rounding,
            9,
            12,
            arc_step,
        );
        push_arc(
            &mut points,
            Vec2 {
                x: max.x - rounding,
                y: max.y - rounding,
            },
            rounding,
            0,
            3,
            arc_step,
        );
        push_arc(
            &mut points,
            Vec2 {
                x: min.x + rounding,
                y: max.y - rounding,
            },
            rounding,
            3,
            6,
            arc_step,
        );
        self.add_convex_poly_filled_aa(&points, col);
    }

    /// `ImDrawList::PathRect` for a fully rounded rectangle: the corner arc points, ready to fill.
    pub fn path_rect_rounded(min: Vec2, max: Vec2, rounding: f32) -> Vec<Vec2> {
        let mut rounding = rounding;
        rounding = rounding.min((max.x - min.x).abs() * 0.5 - 1.0);
        rounding = rounding.min((max.y - min.y).abs() * 0.5 - 1.0);
        let mut points: Vec<Vec2> = Vec::new();
        if rounding < 0.5 {
            points.push(min);
            points.push(Vec2 { x: max.x, y: min.y });
            points.push(max);
            points.push(Vec2 { x: min.x, y: max.y });
            return points;
        }
        let arc_step = arc_step_for_radius(rounding);
        push_arc(
            &mut points,
            Vec2 {
                x: min.x + rounding,
                y: min.y + rounding,
            },
            rounding,
            6,
            9,
            arc_step,
        );
        push_arc(
            &mut points,
            Vec2 {
                x: max.x - rounding,
                y: min.y + rounding,
            },
            rounding,
            9,
            12,
            arc_step,
        );
        push_arc(
            &mut points,
            Vec2 {
                x: max.x - rounding,
                y: max.y - rounding,
            },
            rounding,
            0,
            3,
            arc_step,
        );
        push_arc(
            &mut points,
            Vec2 {
                x: min.x + rounding,
                y: max.y - rounding,
            },
            rounding,
            3,
            6,
            arc_step,
        );
        points
    }

    /// `ImDrawList::_AddLine` for the case the reference's texture selection resolves to the white
    /// pixel, which is what the default atlas produces here: a quad from the segment and its
    /// normal, with no end caps.
    pub fn add_line_aa(&mut self, p1: Vec2, p2: Vec2, col: ImU32, thickness: f32) {
        const A_MASK: u32 = 0xFF00_0000;
        if (col & A_MASK) == 0 {
            return;
        }

        let mut dir_x = p2.x - p1.x;
        let mut dir_y = p2.y - p1.y;
        let d2 = dir_x * dir_x + dir_y * dir_y;
        let inv_len = if d2 > 0.0 { rsqrt_precise(d2) } else { 0.0 };
        dir_x *= inv_len;
        dir_y *= inv_len;
        let norm_x = -dir_y;
        let norm_y = dir_x;

        // `_SelectLineTexture` reports no fringe texture for this pipeline.
        let fringe = 0.0f32;
        let thickness0 = (thickness + fringe) * 0.5;
        let thickness1 = (thickness + fringe) - thickness0;

        let base = self.vtx_buffer.len() as u32;
        for (pos, uv) in [
            (
                Vec2 {
                    x: p1.x - norm_x * thickness0,
                    y: p1.y - norm_y * thickness0,
                },
                self.white_uv,
            ),
            (
                Vec2 {
                    x: p1.x + norm_x * thickness1,
                    y: p1.y + norm_y * thickness1,
                },
                self.white_uv,
            ),
            (
                Vec2 {
                    x: p2.x - norm_x * thickness0,
                    y: p2.y - norm_y * thickness0,
                },
                self.white_uv,
            ),
            (
                Vec2 {
                    x: p2.x + norm_x * thickness1,
                    y: p2.y + norm_y * thickness1,
                },
                self.white_uv,
            ),
        ] {
            self.vtx_buffer.push(DrawVert { pos, uv, col });
        }
        for i in [0u32, 2, 3, 0, 3, 1] {
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

/// `ImRsqrtPrecise`: on x86_64 the reference uses the SSE reciprocal-square-root estimate followed
/// by one Newton step, so the last bits matter and a plain `1/sqrt` can differ. Mirror it with the
/// same intrinsic where it exists, and fall back to `1/sqrt` elsewhere.
fn rsqrt_precise(x: f32) -> f32 {
    // The reference can use the SSE reciprocal-square-root estimate, but that estimate is
    // microarchitecture dependent: two machines produce different last bits, and the golden
    // comparison cannot survive it. The conformance driver is compiled with IMGUI_DISABLE_SSE so
    // both sides take this exact path, which is portable. See tools/conformance/gen-golden.ps1.
    1.0 / x.sqrt()
}
/// `IM_DRAWLIST_ARCFAST_SAMPLE_MAX`: samples around a full circle in the fast arc table.
const ARC_SAMPLE_MAX: i32 = 48;

/// The reference's fast arc table (`ImDrawListSharedData::ArcFastVtx`), pinned from the driver.
/// It is generated at start-up from sin/cos, so it is data, not arithmetic, across compilers.
fn arc_table() -> [Vec2; 48] {
    const BITS: [(u32, u32); 48] = [
        (0x3F80_0000, 0x0000_0000),
        (0x3F7D_CF55, 0x3E05_A8A9),
        (0x3F77_46EA, 0x3E84_83EE),
        (0x3F6C_835E, 0x3EC3_EF16),
        (0x3F5D_B3D7, 0x3F00_0000),
        (0x3F4B_1934, 0x3F1B_D7CB),
        (0x3F35_04F3, 0x3F35_04F3),
        (0x3F1B_D7C9, 0x3F4B_1935),
        (0x3EFF_FFFF, 0x3F5D_B3D8),
        (0x3EC3_EF15, 0x3F6C_835E),
        (0x3E84_83EB, 0x3F77_46EB),
        (0x3E05_A8A4, 0x3F7D_CF55),
        (0xB33B_BD2E, 0x3F80_0000),
        (0xBE05_A8AA, 0x3F7D_CF55),
        (0xBE84_83F2, 0x3F77_46EA),
        (0xBEC3_EF14, 0x3F6C_835F),
        (0xBF00_0001, 0x3F5D_B3D7),
        (0xBF1B_D7CA, 0x3F4B_1934),
        (0xBF35_04F3, 0x3F35_04F3),
        (0xBF4B_1934, 0x3F1B_D7CA),
        (0xBF5D_B3D9, 0x3EFF_FFFA),
        (0xBF6C_8360, 0x3EC3_EF10),
        (0xBF77_46EB, 0x3E84_83EA),
        (0xBF7D_CF54, 0x3E05_A8B1),
        (0xBF80_0000, 0xB3BB_BD2E),
        (0xBF7D_CF54, 0xBE05_A8AD),
        (0xBF77_46EA, 0xBE84_83EF),
        (0xBF6C_835D, 0xBEC3_EF1D),
        (0xBF5D_B3D5, 0xBF00_0003),
        (0xBF4B_1935, 0xBF1B_D7C9),
        (0xBF35_04F4, 0xBF35_04F2),
        (0xBF1B_D7CB, 0xBF4B_1933),
        (0xBEFF_FFFD, 0xBF5D_B3D8),
        (0xBEC3_EF0B, 0xBF6C_8361),
        (0xBE84_83EC, 0xBF77_46EB),
        (0xBE05_A896, 0xBF7D_CF55),
        (0x324C_DE2E, 0xBF80_0000),
        (0x3E05_A8B7, 0xBF7D_CF54),
        (0x3E84_83ED, 0xBF77_46EA),
        (0x3EC3_EF1B, 0xBF6C_835D),
        (0x3F00_0006, 0xBF5D_B3D4),
        (0x3F1B_D7CC, 0xBF4B_1933),
        (0x3F35_04F7, 0xBF35_04EF),
        (0x3F4B_1935, 0xBF1B_D7C9),
        (0x3F5D_B3DA, 0xBEFF_FFF8),
        (0x3F6C_835F, 0xBEC3_EF15),
        (0x3F77_46E9, 0xBE84_83F6),
        (0x3F7D_CF56, 0xBE05_A88B),
    ];
    let mut table = [Vec2 { x: 0.0, y: 0.0 }; 48];
    let mut i = 0;
    while i < 48 {
        table[i] = Vec2 {
            x: f32::from_bits(BITS[i].0),
            y: f32::from_bits(BITS[i].1),
        };
        i += 1;
    }
    table
}

/// `IM_DRAWLIST_CIRCLE_AUTO_SEGMENT_CALC`: the number of segments the reference picks for a circle
/// of this radius, rounded up to an even number and clamped to [4, 512]. The default tessellation
/// error is 0.30 (`ImGuiStyle::CircleTessellationMaxError`).
pub fn circle_segment_count(radius: f32, max_error: f32) -> i32 {
    const MIN: i32 = 4;
    const MAX: i32 = 512;
    if radius <= 0.0 {
        return ARC_SAMPLE_MAX;
    }
    let error = max_error.min(radius);
    let angle = (1.0f32 - error / radius).acos();
    let raw = (core::f32::consts::PI / angle).ceil() as i32;
    let even = ((raw + 1) / 2) * 2;
    even.clamp(MIN, MAX)
}

/// `CalcCornerArcStepAndCount`: the sampling step within a quarter circle for a given segment count.
pub fn arc_step_for_segments(segment_count: i32) -> i32 {
    const QUARTER: i32 = ARC_SAMPLE_MAX / 4;
    let mut step = (ARC_SAMPLE_MAX / segment_count.max(1)).clamp(1, QUARTER);
    let count = QUARTER / step;
    if step * count != QUARTER {
        step = QUARTER / count.max(1);
    }
    step
}

/// The sampling step the reference uses for a corner of this radius.
pub fn arc_step_for_radius(radius: f32) -> i32 {
    arc_step_for_segments(circle_segment_count(radius, 0.30))
}
/// `ImDrawList::_PathArcToFastEx`: sample the fast arc table between two 12ths of a circle.
fn push_arc(
    out: &mut Vec<Vec2>,
    center: Vec2,
    radius: f32,
    min_of_12: i32,
    max_of_12: i32,
    arc_step: i32,
) {
    if radius < 0.5 {
        out.push(center);
        return;
    }

    let min_sample = min_of_12 * ARC_SAMPLE_MAX / 12;
    let max_sample = max_of_12 * ARC_SAMPLE_MAX / 12;
    let range = (max_sample - min_sample).abs();

    let mut first_step = arc_step.max(1);
    let next_step = first_step;
    if arc_step > 1 {
        let overstep = range % arc_step;
        if overstep > 0 && range > 0 {
            first_step = arc_step - (arc_step - overstep) / 2;
        }
    }

    let table = arc_table();
    let mut sample_index = min_sample.rem_euclid(ARC_SAMPLE_MAX);
    let mut a = min_sample;
    let mut step = first_step;
    let ascending = min_sample <= max_sample;
    loop {
        if ascending && a > max_sample {
            break;
        }
        if !ascending && a < max_sample {
            break;
        }
        let index = sample_index.rem_euclid(ARC_SAMPLE_MAX) as usize;
        let sample = table[index];
        out.push(Vec2 {
            x: center.x + sample.x * radius,
            y: center.y + sample.y * radius,
        });
        if ascending {
            a += step;
            sample_index += step;
        } else {
            a -= step;
            sample_index -= step;
        }
        step = next_step;
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
