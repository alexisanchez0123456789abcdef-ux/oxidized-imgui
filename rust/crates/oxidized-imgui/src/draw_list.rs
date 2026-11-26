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

/// `ImDrawListSharedData::TexUvWhitePixel` for the default atlas (a 1x1 white pixel).
pub const DEFAULT_WHITE_UV: Vec2 = Vec2 {
    x: 0.000_976_562_5,
    y: 0.003_906_25,
};

/// A packed RGBA color, `ImU32`.
pub type ImU32 = u32;

/// `ImDrawFlags`: the per-call switches of the primitive functions. The list also carries a set of
/// defaults in the same bit space (`ImDrawListFlags_AntiAliasedLines` **is** `ImDrawFlags_AALines`),
/// which is why the two are OR-ed together before use.
pub mod draw_flags {
    pub const CLOSED: u32 = 1 << 9;
    pub const JOIN_MITER: u32 = 1 << 10;
    pub const CAP_SQUARE: u32 = 1 << 11;
    /// `ImDrawListFlags_AntiAliasedFill`.
    pub const AAFILL: u32 = 1 << 12;
    /// `ImDrawListFlags_AntiAliasedLines`: the default of a window's draw list.
    pub const AALINES: u32 = 1 << 13;
    /// `ImDrawListFlags_AntiAliasedLineEnds`.
    pub const AALINE_ENDS: u32 = 1 << 14;
    /// `ImDrawFlags_UseTexForRoundCorners`: rounded corners may sample the baked corner texture.
    pub const USE_TEX_FOR_ROUND_CORNERS: u32 = 1 << 21;
    /// `ImDrawFlags_AllowTexForRoundCorners_`: internal companion of the flag above.
    pub const ALLOW_TEX_FOR_ROUND_CORNERS: u32 = 1 << 23;
    pub const STROKE_INSIDE: u32 = 1 << 17;
    pub const STROKE_CENTER: u32 = 2 << 17;
    pub const STROKE_CENTER_BIASED: u32 = 3 << 17;
    pub const STROKE_OUTSIDE: u32 = 4 << 17;
    pub const STROKE_LEGACY: u32 = 7 << 17;
    pub const STROKE_MASK: u32 = 7 << 17;
}

/// The reference writes `0.7071f` literally when it needs a diagonal. That is **not** an exact
/// `1/sqrt(2)`, and the difference shows up in the low bits of the geometry, so it stays as the
/// reference wrote it instead of a "better" constant.
#[allow(clippy::approx_constant)]
pub(crate) const REF_0_7071: f32 = 0.7071;

/// `_GetStrokePos`: the stroke position a call asked for, or the caller's default.
fn stroke_pos(flags: u32, default: u32) -> u32 {
    if flags & draw_flags::STROKE_MASK != 0 {
        flags & draw_flags::STROKE_MASK
    } else {
        default
    }
}

/// `ImDrawListSharedData::InitialDrawFlags` for the default style: what a window's draw list starts
/// each frame with. Anti-aliased fills and lines, textured rounded corners; **not** anti-aliased
/// line ends, which the default style leaves off.
pub const WINDOW_DRAW_FLAGS: u32 = draw_flags::AAFILL
    | draw_flags::AALINES
    | draw_flags::USE_TEX_FOR_ROUND_CORNERS
    | draw_flags::ALLOW_TEX_FOR_ROUND_CORNERS;

/// `ImAlphaMultiply`: scales the alpha of a colour, used for sub-pixel thick strokes.
fn alpha_multiply(col: ImU32, alpha_mul: f32) -> ImU32 {
    const A_SHIFT: u32 = 24;
    const A_MASK: u32 = 0xFF00_0000;
    let a = (col & A_MASK) >> A_SHIFT;
    let a = (a as f32 * alpha_mul) as u32;
    (col & !A_MASK) | (a << A_SHIFT)
}

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
    /// `ImDrawListSharedData::TexUvLines`: the atlas line-texture table, one entry per baked
    /// thickness as `[u0, u1, v, 1/width]`. Empty until the atlas is built, like the reference's
    /// null pointer; the harness pins it from a fixture.
    pub tex_uv_lines: Vec<[f32; 4]>,
    clip_stack: Vec<[f32; 4]>,
    current_clip: [f32; 4],
}

impl DrawList {
    /// Resets the list for a new frame: buffers empty and one command ready, like the reference's
    /// `_ResetForNewFrame` ("We always have a command ready in the buffer").
    pub fn new() -> Self {
        Self {
            white_uv: DEFAULT_WHITE_UV,
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

    /// `_PopUnusedDrawCmd`: drop trailing commands that carry no geometry. The reference does this
    /// when a list is presented, which is why a list never ends with an empty command.
    pub fn pop_unused_draw_cmd(&mut self) {
        while self.cmd_buffer.len() > 1 {
            let last_empty = self
                .cmd_buffer
                .last()
                .map(|cmd| cmd.elem_count == 0)
                .unwrap_or(false);
            if !last_empty {
                break;
            }
            self.cmd_buffer.pop();
        }
    }

    /// `_PopUnusedDrawCmd` without keeping one behind: the draw splitter's merge drops every
    /// trailing command that carries no geometry, and adds one back later if it needs it.
    pub fn pop_all_unused_commands(&mut self) {
        while self
            .cmd_buffer
            .last()
            .map(|cmd| cmd.elem_count == 0)
            .unwrap_or(false)
        {
            self.cmd_buffer.pop();
        }
    }

    /// `ImDrawList::AddDrawCmd` with an explicit clip: opens a command for the indices that follow,
    /// which is how the draw splitter hands each table column its own command.
    pub fn start_command(&mut self, clip: [f32; 4]) {
        let cmd = DrawCmd {
            clip_rect: clip,
            idx_offset: self.idx_buffer.len() as u32,
            ..DrawCmd::default()
        };
        self.cmd_buffer.push(cmd);
    }

    /// Sets the current clip on the command being built, without opening a new one. The window
    /// draw list starts with the fullscreen clip before its background is drawn.
    pub fn set_current_clip(&mut self, rect: [f32; 4]) {
        self.current_clip = rect;
        if let Some(cmd) = self.cmd_buffer.last_mut() {
            cmd.clip_rect = rect;
        }
    }

    /// The clip the command being built carries (`ImDrawList::_CmdHeader.ClipRect`). A caller that
    /// wants to push the clip already in effect needs it.
    pub fn current_clip_rect(&self) -> [f32; 4] {
        self.current_clip
    }

    /// What closing a window does to the command list, pinned by `window_bg` and the full scene:
    /// the empty command left by the clip push at the start is dropped, and the clip pop opens a
    /// new empty command when the current one has geometry, and that one stays.
    pub fn finish_clip_scope(&mut self) {
        let had_empty = self.cmd_buffer.len() > 1
            && self
                .cmd_buffer
                .last()
                .map(|c| c.elem_count == 0)
                .unwrap_or(false);
        if had_empty {
            self.cmd_buffer.pop();
        } else if self
            .cmd_buffer
            .last()
            .map(|c| c.elem_count > 0)
            .unwrap_or(false)
        {
            let cmd = DrawCmd {
                clip_rect: self.fullscreen_clip,
                idx_offset: self.idx_buffer.len() as u32,
                ..DrawCmd::default()
            };
            self.cmd_buffer.push(cmd);
        }
        self.current_clip = self.fullscreen_clip;
    }

    /// `ImDrawList::_OnChangedClipRect`: what a clip change does to the command buffer.
    ///
    /// Three outcomes, in the reference's order: a command that already has geometry is closed and
    /// a new one opened **only when the clip really changed**; an empty command left by a clip
    /// push/pop that drew nothing is dropped when the previous command already carries the restored
    /// clip, so the geometry can continue there; otherwise the current command takes the new clip.
    fn on_changed_clip_rect(&mut self) {
        // `ImDrawCmd_HeaderCompare` also compares `TexRef` and `VtxOffset`; both are the defaults on
        // every command here (no texture changes yet), so the clip is what can differ.
        let Some(&curr) = self.cmd_buffer.last() else {
            return;
        };
        if curr.elem_count != 0 && curr.clip_rect != self.current_clip {
            // A new command carries the index offset of the buffer when it opens, as
            // `ImDrawList::AddDrawCmd` does. `VtxOffset` stays 0 until a backend asks for it.
            let cmd = DrawCmd {
                clip_rect: self.current_clip,
                idx_offset: self.idx_buffer.len() as u32,
                ..DrawCmd::default()
            };
            self.cmd_buffer.push(cmd);
            return;
        }
        if curr.elem_count == 0 && self.cmd_buffer.len() > 1 {
            let prev = self.cmd_buffer[self.cmd_buffer.len() - 2];
            // `ImDrawCmd_AreSequentialIdxOffset`: the empty command follows the previous one with no
            // geometry in between, so merging it back loses nothing.
            let sequential = prev.idx_offset + prev.elem_count == curr.idx_offset;
            if prev.clip_rect == self.current_clip && sequential && prev.user_callback.is_none() {
                self.cmd_buffer.pop();
                return;
            }
        }
        if let Some(cmd) = self.cmd_buffer.last_mut() {
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
        self.add_text_in(font, size, pos, col, text, None, None);
    }

    /// `ImDrawList::AddText` with a `cpu_fine_clip_rect`: the same, and the cut glyphs also have
    /// their texture coordinates trimmed.
    pub fn add_text_clipped(
        &mut self,
        font: &crate::font::Font,
        size: f32,
        pos: Vec2,
        col: ImU32,
        text: &str,
        clip: (Vec2, Vec2),
    ) {
        self.add_text_in(font, size, pos, col, text, None, Some(clip));
    }

    /// The body of `ImFont::RenderText`: `base_clip` is the rectangle the reference takes as an
    /// argument (`window->ClipRect` when called through `ImDrawList::AddText`), and `fine_clip` is
    /// the optional `cpu_fine_clip_rect`, which is intersected with it and also trims the texture
    /// coordinates of the glyphs it cuts.
    ///
    /// A glyph whose quad falls outside the rectangle is dropped either way; one that straddles an
    /// edge has its position *and its texture coordinates* trimmed only with a fine clip, so the
    /// part outside does not bleed in.
    ///
    /// Single line only: the reference also handles `'\n'`, word wrap and the multi-line
    /// fast-forward, none of which the port needs yet.
    #[allow(clippy::too_many_arguments)]
    pub fn add_text_in(
        &mut self,
        font: &crate::font::Font,
        size: f32,
        pos: Vec2,
        col: ImU32,
        text: &str,
        base_clip: Option<[f32; 4]>,
        fine_clip: Option<(Vec2, Vec2)>,
    ) {
        let scale = size / font.size;
        // The effective clip: the rectangle the caller works with, narrowed by the fine clip.
        let mut clip = base_clip.unwrap_or(self.current_clip);
        if let Some((min, max)) = fine_clip {
            clip[0] = clip[0].max(min.x);
            clip[1] = clip[1].max(min.y);
            clip[2] = clip[2].min(max.x);
            clip[3] = clip[3].min(max.y);
        }
        let cpu_fine_clip = fine_clip.is_some();
        // `IM_TRUNC(pos)`: text is snapped to whole pixels unless the list asks not to.
        let mut x = pos.x.trunc();
        let y = pos.y.trunc();
        if y > clip[3] {
            return;
        }
        // Fast-forward to the first visible line. The port draws one line, so this consumes it.
        if y + size < clip[1] {
            return;
        }
        for ch in text.chars() {
            let Some(glyph) = font.glyph(ch) else {
                // `FindGlyph` never fails in the reference: a character the atlas does not have
                // gets the fallback glyph, which draws nothing but still advances the pen.
                x += font.advance_of(ch, size);
                continue;
            };
            let char_width = glyph.advance_x * scale;
            let mut x1 = x + glyph.offset[0] * scale;
            let mut y1 = y + glyph.offset[1] * scale;
            let mut x2 = x + glyph.offset[2] * scale;
            let mut y2 = y + glyph.offset[3] * scale;
            // A glyph with no area has nothing to draw, which is how the reference's `Visible` flag
            // reports the space.
            let visible = x2 > x1 && y2 > y1;
            if visible && x1 <= clip[2] && x2 >= clip[0] {
                let mut u1 = glyph.uv[0];
                let mut v1 = glyph.uv[1];
                let mut u2 = glyph.uv[2];
                let mut v2 = glyph.uv[3];
                if cpu_fine_clip {
                    if x1 < clip[0] {
                        u1 += (1.0 - (x2 - clip[0]) / (x2 - x1)) * (u2 - u1);
                        x1 = clip[0];
                    }
                    if y1 < clip[1] {
                        v1 += (1.0 - (y2 - clip[1]) / (y2 - y1)) * (v2 - v1);
                        y1 = clip[1];
                    }
                    if x2 > clip[2] {
                        u2 = u1 + ((clip[2] - x1) / (x2 - x1)) * (u2 - u1);
                        x2 = clip[2];
                    }
                    if y2 > clip[3] {
                        v2 = v1 + ((clip[3] - y1) / (y2 - y1)) * (v2 - v1);
                        y2 = clip[3];
                    }
                    if y1 >= y2 {
                        x += char_width;
                        continue;
                    }
                }
                self.prim_quad_uv(
                    Vec2 { x: x1, y: y1 },
                    Vec2 { x: x2, y: y1 },
                    Vec2 { x: x2, y: y2 },
                    Vec2 { x: x1, y: y2 },
                    Vec2 { x: u1, y: v1 },
                    Vec2 { x: u2, y: v1 },
                    Vec2 { x: u2, y: v2 },
                    Vec2 { x: u1, y: v2 },
                    col,
                );
            }
            x += char_width;
        }
    }
    /// `ImDrawList::AddConvexPolyFilled` with anti-aliasing, miter joins only (the branch a
    /// rectangle takes). Returns the index count emitted.
    ///
    /// Layout matches the reference: the inner ring first, then one outer vertex per edge, with the
    /// first edge's two triangles patched to wrap onto the last outer vertex.
    pub fn add_convex_poly_filled_aa(&mut self, points: &[Vec2], col: ImU32) -> u32 {
        self.add_convex_poly_filled(points, col, 0)
    }

    /// The full `ImDrawList::AddConvexPolyFilled`, anti-aliased.
    ///
    /// `flags` only matters for `ImDrawFlags_JoinMiter`: with it every corner is a plain miter, which
    /// is what `PathFillConvex(col, ImDrawFlags_JoinMiter)` asks for (circles and rounded
    /// rectangles). Without it a corner whose miter would overshoot gets a bevel, up to two of them,
    /// which is the path a rectangle fill takes.
    pub fn add_convex_poly_filled(&mut self, points: &[Vec2], col: ImU32, flags: u32) -> u32 {
        const MITER_ANGLE_LIMIT: f32 = -0.999_961_9;
        const MITER_LIMIT_SQR: f32 = 4.0 * 4.0;
        const MAX_BEVELS: i32 = 2;
        const A_MASK: u32 = 0xFF00_0000;

        let n = points.len();
        if n < 3 || (col & A_MASK) == 0 {
            return 0;
        }
        let miters_only = flags & draw_flags::JOIN_MITER != 0;
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
            let bevel = !miters_only && miter_distance_sqr > MITER_LIMIT_SQR;
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

    /// `IM_HAS_ROUNDING`: a radius this small is drawn as a point, not as a polygon.
    fn has_rounding(&self, radius: f32) -> bool {
        radius >= 0.25 * self.fringe_scale
    }

    /// `ImDrawList::_CalcCircleAutoSegmentCount`: the cached count for this radius, or the formula
    /// past the table. The radius is scaled back to screen pixels before the lookup.
    fn calc_circle_auto_segment_count(&self, radius: f32) -> i32 {
        let radius = radius * (1.0 / self.fringe_scale);
        let radius_idx = (radius + 0.999) as i32;
        if (0..CIRCLE_SEGMENT_COUNTS.len() as i32).contains(&radius_idx) {
            return CIRCLE_SEGMENT_COUNTS[radius_idx as usize] as i32;
        }
        circle_segment_count(radius, CIRCLE_TESS_MAX_ERROR)
    }

    /// `ImDrawList::_PathArcToFastEx`: the sampled points of an arc. `a_step <= 0` asks for the
    /// automatic step of this radius.
    fn path_arc_to_fast_ex(
        &self,
        out: &mut Vec<Vec2>,
        center: Vec2,
        radius: f32,
        a_min_sample: i32,
        a_max_sample: i32,
        a_step: i32,
    ) {
        if !self.has_rounding(radius) {
            out.push(center);
            return;
        }
        let step = if a_step <= 0 {
            corner_arc_step_and_count(self.calc_circle_auto_segment_count(radius)).0
        } else {
            a_step.clamp(1, ARC_SAMPLE_MAX / 4)
        };
        push_arc_samples(out, center, radius, a_min_sample, a_max_sample, step);
    }

    /// `ImDrawList::_PathArcToN`: `num_segments` even steps between the two angles, both ends
    /// included. This is the path that needs `cos`/`sin`, so no golden can pin it across platforms
    /// yet.
    fn path_arc_to_n(
        &self,
        out: &mut Vec<Vec2>,
        center: Vec2,
        radius: f32,
        a_min: f32,
        a_max: f32,
        num_segments: i32,
    ) {
        if !self.has_rounding(radius) {
            out.push(center);
            return;
        }
        for i in 0..=num_segments {
            let a = a_min + (i as f32 / num_segments as f32) * (a_max - a_min);
            out.push(Vec2 {
                x: center.x + a.cos() * radius,
                y: center.y + a.sin() * radius,
            });
        }
    }

    /// `ImDrawList::PathArcTo`: explicit segments, the fast arc table below the cutoff radius and
    /// exact trigonometry above it. The cutoff branch is the one a circle takes.
    fn path_arc_to(
        &self,
        out: &mut Vec<Vec2>,
        center: Vec2,
        radius: f32,
        a_min: f32,
        a_max: f32,
        num_segments: i32,
    ) {
        if !self.has_rounding(radius) {
            out.push(center);
            return;
        }
        if num_segments > 0 {
            self.path_arc_to_n(out, center, radius, a_min, a_max, num_segments);
            return;
        }
        if radius <= ARC_FAST_RADIUS_CUTOFF {
            let a_is_reverse = a_max < a_min;
            let auto_seg_count = self.calc_circle_auto_segment_count(radius);
            let (arc_step, arc_step_count) = corner_arc_step_and_count(auto_seg_count);
            let arc_seg_count = arc_step_count * 4;
            let two_pi = core::f32::consts::PI * 2.0;
            let a_min_sample_f = arc_seg_count as f32 * a_min / two_pi;
            let a_max_sample_f = arc_seg_count as f32 * a_max / two_pi;
            let a_min_sample = (if a_is_reverse {
                a_min_sample_f.floor()
            } else {
                a_min_sample_f.ceil()
            }) as i32
                * arc_step;
            let a_max_sample = (if a_is_reverse {
                a_max_sample_f.ceil()
            } else {
                a_max_sample_f.floor()
            }) as i32
                * arc_step;
            let a_mid_samples = if a_is_reverse {
                (a_min_sample - a_max_sample).max(0)
            } else {
                (a_max_sample - a_min_sample).max(0)
            };
            let a_min_segment_angle = a_min_sample as f32 * two_pi / ARC_SAMPLE_MAX as f32;
            let a_max_segment_angle = a_max_sample as f32 * two_pi / ARC_SAMPLE_MAX as f32;
            // The exact ends are only emitted when the table sample does not already land on them.
            let a_emit_start = (a_min_segment_angle - a_min).abs() >= 1e-5;
            let a_emit_end = (a_max - a_max_segment_angle).abs() >= 1e-5;
            if a_emit_start {
                out.push(Vec2 {
                    x: center.x + a_min.cos() * radius,
                    y: center.y + a_min.sin() * radius,
                });
            }
            if a_mid_samples > 0 {
                self.path_arc_to_fast_ex(out, center, radius, a_min_sample, a_max_sample, arc_step);
            }
            if a_emit_end {
                out.push(Vec2 {
                    x: center.x + a_max.cos() * radius,
                    y: center.y + a_max.sin() * radius,
                });
            }
        } else {
            let arc_length = (a_max - a_min).abs();
            let circle_segment_count = self.calc_circle_auto_segment_count(radius);
            let arc_segment_count = ((circle_segment_count as f32 * arc_length
                / (core::f32::consts::PI * 2.0))
                .ceil() as i32)
                .max(1);
            self.path_arc_to_n(out, center, radius, a_min, a_max, arc_segment_count);
        }
    }

    /// `ImDrawList::AddCircleFilled`: a filled disc with the automatic segment count when
    /// `num_segments` is 0. Below `ArcFastRadiusCutoff` it samples the fast arc table, which is why
    /// it needs no trigonometry.
    pub fn add_circle_filled(&mut self, center: Vec2, radius: f32, col: ImU32, num_segments: i32) {
        const A_MASK: u32 = 0xFF00_0000;
        if (col & A_MASK) == 0 {
            return;
        }
        // The smallest radius whose edge fringe does not overlap itself.
        let min_radius = self.fringe_scale * REF_1_41421356 * 0.5;
        let mut radius = radius;
        let mut col = col;
        if radius < min_radius {
            if radius < (1.0 / 255.0) * min_radius {
                return;
            }
            col = alpha_multiply(col, radius / min_radius);
            radius = min_radius;
        }

        let mut path: Vec<Vec2> = Vec::new();
        let mut num_segments = num_segments;
        if num_segments <= 0 && radius <= ARC_FAST_RADIUS_CUTOFF {
            self.path_arc_to_fast_ex(&mut path, center, radius, 0, ARC_SAMPLE_MAX, 0);
            // The full circle ends where it starts: the reference drops the duplicate point.
            path.pop();
        } else {
            if num_segments <= 0 {
                num_segments = self.calc_circle_auto_segment_count(radius);
            }
            num_segments = num_segments.clamp(CIRCLE_SEGMENT_MIN, CIRCLE_SEGMENT_MAX);
            // A closed shape repeats no vertex, so the last segment is left out of the count.
            let a_max =
                core::f32::consts::PI * 2.0 * (num_segments as f32 - 1.0) / num_segments as f32;
            self.path_arc_to(&mut path, center, radius, 0.0, a_max, num_segments - 1);
        }
        self.add_convex_poly_filled(&path, col, draw_flags::JOIN_MITER);
    }

    /// `ImDrawList::AddCircle` with the default inside stroke position.
    pub fn add_circle(
        &mut self,
        center: Vec2,
        radius: f32,
        col: ImU32,
        num_segments: i32,
        thickness: f32,
    ) {
        self.add_circle_ex(center, radius, col, num_segments, thickness, 0);
    }

    /// `ImDrawList::AddCircle` with explicit flags: the outline of a circle, which is a closed
    /// polyline over the same path a fill uses.
    pub fn add_circle_ex(
        &mut self,
        center: Vec2,
        radius: f32,
        col: ImU32,
        num_segments: i32,
        thickness: f32,
        flags_in: u32,
    ) {
        const A_MASK: u32 = 0xFF00_0000;
        if (col & A_MASK) == 0 {
            return;
        }
        let flags = flags_in | self.flags;
        let mut stroke = stroke_pos(flags, draw_flags::STROKE_INSIDE);
        let mut radius = radius;
        if stroke == draw_flags::STROKE_LEGACY {
            radius -= 0.5;
            stroke = draw_flags::STROKE_CENTER;
        }
        radius = radius.max(0.01);

        let outer_radius = if stroke == draw_flags::STROKE_CENTER {
            radius + thickness * 0.5
        } else if stroke == draw_flags::STROKE_CENTER_BIASED {
            radius + self.center_biased_offset(thickness)
        } else if stroke == draw_flags::STROKE_OUTSIDE {
            radius + thickness
        } else {
            radius
        };
        if outer_radius <= thickness {
            // The stroke swallowed the circle: the reference draws a filled disc instead.
            self.add_circle_filled(center, outer_radius, col, num_segments);
            return;
        }

        let mut path: Vec<Vec2> = Vec::new();
        let mut num_segments = num_segments;
        if num_segments <= 0 && outer_radius <= ARC_FAST_RADIUS_CUTOFF {
            // The step comes from the outer radius, to agree with a rounded rectangle of that size.
            let a_step = ARC_SAMPLE_MAX / self.calc_circle_auto_segment_count(outer_radius);
            self.path_arc_to_fast_ex(&mut path, center, radius, 0, ARC_SAMPLE_MAX, a_step);
            path.pop();
        } else {
            if num_segments <= 0 {
                num_segments = self.calc_circle_auto_segment_count(outer_radius);
            }
            num_segments = num_segments.clamp(3, CIRCLE_SEGMENT_MAX);
            let a_max =
                core::f32::consts::PI * 2.0 * (num_segments as f32 - 1.0) / num_segments as f32;
            self.path_arc_to(&mut path, center, radius, 0.0, a_max, num_segments - 1);
        }

        if path.len() >= 3 {
            // Clamp the inner stroke expansion so it cannot fold over the edge of the polygon.
            let edge_dist = radius * approx_apothem(path.len() as i32);
            self.add_polyline_max(
                &path,
                col,
                thickness,
                draw_flags::CLOSED | draw_flags::JOIN_MITER | stroke,
                edge_dist,
            );
        }
    }

    /// `ImDrawList::AddLine` with the line textures of the atlas, which is the path a draw list
    /// takes when anti-aliased strokes are enabled (the default for a window).
    pub fn add_line_aa(&mut self, p1: Vec2, p2: Vec2, col: ImU32, thickness: f32) {
        self.add_line(p1, p2, col, thickness, 0);
    }

    /// `ImDrawList::_AddPolyline` for an **open** polyline with miter joins and simple caps: the
    /// combination the `aa_polyline` scenario pins.
    pub fn add_polyline_aa_miter(&mut self, points: &[Vec2], col: ImU32, thickness: f32) {
        self.add_polyline(points, col, thickness, draw_flags::JOIN_MITER);
    }

    /// `ImDrawList::_AddPolyline` for a **closed** path with miter joins and the inside stroke
    /// position, which is what a border (`AddRect` with defaults) takes.
    pub fn add_closed_polyline_aa_miter_inside(
        &mut self,
        points: &[Vec2],
        col: ImU32,
        thickness: f32,
    ) {
        self.add_polyline(
            points,
            col,
            thickness,
            draw_flags::CLOSED | draw_flags::JOIN_MITER | draw_flags::STROKE_INSIDE,
        );
    }

    /// `ImDrawList::AddRect` for a rectangle **without rounding**, which is the shape a table's
    /// outer border takes.
    ///
    /// Two paths, as in the reference: when the coordinates, the thickness and the pixel density are
    /// all whole numbers it emits eight vertices and twenty-four indices (the four sides as quads,
    /// white texture, opaque colour); otherwise it falls back to the closed inside-stroked polyline.
    /// A rectangle thinner than its stroke collapses into a filled one.
    pub fn add_rect_stroke(&mut self, min: Vec2, max: Vec2, col: ImU32, thickness: f32) {
        const A_MASK: u32 = 0xFF00_0000;
        if (col & A_MASK) == 0 {
            return;
        }
        let flags = self.flags;
        let stroke = stroke_pos(flags, draw_flags::STROKE_INSIDE);
        if stroke == draw_flags::STROKE_LEGACY {
            let half = if flags & draw_flags::AALINES != 0 {
                0.50
            } else {
                0.49
            };
            let points = [
                Vec2 {
                    x: min.x + 0.50,
                    y: min.y + 0.50,
                },
                Vec2 {
                    x: max.x - 0.50,
                    y: min.y + 0.50,
                },
                Vec2 {
                    x: max.x - 0.50,
                    y: max.y - half,
                },
                Vec2 {
                    x: min.x + 0.50,
                    y: max.y - half,
                },
            ];
            self.add_polyline_max(
                &points,
                col,
                thickness,
                draw_flags::CLOSED | draw_flags::JOIN_MITER | draw_flags::STROKE_CENTER,
                f32::MAX,
            );
            return;
        }

        // The outer boundary of the stroke depends on where the stroke sits.
        let offset = if stroke == draw_flags::STROKE_CENTER {
            thickness * 0.5
        } else if stroke == draw_flags::STROKE_CENTER_BIASED {
            self.center_biased_offset(thickness)
        } else if stroke == draw_flags::STROKE_OUTSIDE {
            thickness
        } else {
            0.0
        };
        let outer_min = Vec2 {
            x: min.x - offset,
            y: min.y - offset,
        };
        let outer_max = Vec2 {
            x: max.x + offset,
            y: max.y + offset,
        };
        let half_min_dim = ((outer_max.x - outer_min.x).min(outer_max.y - outer_min.y)) * 0.5;
        if half_min_dim <= thickness {
            self.add_rect_filled(outer_min, outer_max, col);
            return;
        }

        let inv_fringe = 1.0 / self.fringe_scale;
        let is_truncated = outer_min.x.fract() == 0.0
            && outer_min.y.fract() == 0.0
            && outer_max.x.fract() == 0.0
            && outer_max.y.fract() == 0.0
            && thickness.fract() == 0.0;
        let fringe_is_integer = inv_fringe.fract() == 0.0;
        if fringe_is_integer && is_truncated {
            let screen_thickness = (thickness * inv_fringe) as i32;
            if screen_thickness <= 0 {
                return;
            }
            // Pixel aligned non-rounded rectangle: one quad per side, shared corners.
            let t = screen_thickness as f32 * self.fringe_scale;
            let uv = self.white_uv;
            let base = self.vtx_buffer.len() as u32;
            for (pos, col) in [
                (outer_min, col),
                (
                    Vec2 {
                        x: outer_min.x + t,
                        y: outer_min.y + t,
                    },
                    col,
                ),
                (
                    Vec2 {
                        x: outer_max.x,
                        y: outer_min.y,
                    },
                    col,
                ),
                (
                    Vec2 {
                        x: outer_max.x - t,
                        y: outer_min.y + t,
                    },
                    col,
                ),
                (outer_max, col),
                (
                    Vec2 {
                        x: outer_max.x - t,
                        y: outer_max.y - t,
                    },
                    col,
                ),
                (
                    Vec2 {
                        x: outer_min.x,
                        y: outer_max.y,
                    },
                    col,
                ),
                (
                    Vec2 {
                        x: outer_min.x + t,
                        y: outer_max.y - t,
                    },
                    col,
                ),
            ] {
                self.vtx_buffer.push(DrawVert { pos, uv, col });
            }
            for tri in [
                [0u32, 3, 1],
                [0, 2, 3],
                [2, 5, 3],
                [2, 4, 5],
                [4, 7, 5],
                [4, 6, 7],
                [6, 1, 7],
                [6, 0, 1],
            ] {
                for i in tri {
                    self.idx_buffer.push((base + i) as ImDrawIdx);
                }
            }
            if let Some(cmd) = self.cmd_buffer.last_mut() {
                cmd.elem_count += 24;
            }
            return;
        }

        let points = [
            outer_min,
            Vec2 {
                x: outer_max.x,
                y: outer_min.y,
            },
            outer_max,
            Vec2 {
                x: outer_min.x,
                y: outer_max.y,
            },
        ];
        self.add_polyline_max(
            &points,
            col,
            thickness,
            draw_flags::CLOSED | draw_flags::JOIN_MITER | draw_flags::STROKE_INSIDE,
            half_min_dim,
        );
    }

    /// `ImDrawList::AddRect` with defaults: a closed miter stroke placed inside the rectangle.
    pub fn add_rect_border_inside_aa(&mut self, min: Vec2, max: Vec2, col: ImU32, thickness: f32) {
        let points = [
            min,
            Vec2 { x: max.x, y: min.y },
            max,
            Vec2 { x: min.x, y: max.y },
        ];
        // `AddRect` clamps the inner stroke expansion to half the smaller side.
        let half_min_dim = (max.x - min.x).abs().min((max.y - min.y).abs()) * 0.5;
        self.add_polyline_max(
            &points,
            col,
            thickness,
            draw_flags::CLOSED | draw_flags::JOIN_MITER | draw_flags::STROKE_INSIDE,
            half_min_dim,
        );
    }

    /// `ImDrawList::AddPolyline`, which is `_AddPolyline` with no clamp on the inner stroke.
    pub fn add_polyline(&mut self, points: &[Vec2], col: ImU32, thickness: f32, flags: u32) {
        self.add_polyline_max(points, col, thickness, flags, f32::MAX);
    }

    /// `ImGui::RenderArrowEx`: the triangle an arrow button and a window icon draw.
    ///
    /// `cos_a`/`sin_a` are the direction of the arrow (`g_ImGuiDirCosSin`), `scale` multiplies the
    /// radius and `arrow_th` is the stroke weight relative to it: `0.0` fills the triangle instead
    /// of stroking it. `font_size` is `ImDrawListSharedData::FontSize`, the radius reference.
    #[allow(clippy::too_many_arguments)]
    pub fn add_arrow(
        &mut self,
        pos: Vec2,
        col: ImU32,
        cos_a: f32,
        sin_a: f32,
        scale: f32,
        font_size: f32,
        arrow_sizes: Vec2,
        arrow_th: f32,
    ) {
        let is_fill = arrow_th <= 0.0;
        let r = font_size * 0.50 * scale;
        let thickness = if is_fill {
            0.0
        } else {
            // `IM_ROUND(ImClamp(r * arrow_th, 1.0f, r * 0.5f))`.
            let clamped = (r * arrow_th).max(1.0).min(r * 0.5);
            ((clamped + 0.5) as i32) as f32
        };

        // Bias so the shape looks centred in its box.
        let bias = if is_fill {
            r * 0.05
        } else {
            thickness * 0.8 * REF_0_7071
        };
        let center = Vec2 {
            x: pos.x + r + bias * cos_a,
            y: pos.y + r + bias * sin_a,
        };

        let arrow_h = arrow_sizes.y * r;
        let arrow_w = arrow_sizes.x * r;
        let mut p = [
            Vec2 {
                x: center.x - arrow_h * cos_a - arrow_w * -sin_a,
                y: center.y - arrow_h * sin_a - arrow_w * cos_a,
            },
            Vec2 {
                x: center.x + arrow_h * cos_a,
                y: center.y + arrow_h * sin_a,
            },
            Vec2 {
                x: center.x - arrow_h * cos_a + arrow_w * -sin_a,
                y: center.y - arrow_h * sin_a + arrow_w * cos_a,
            },
        ];

        // Pixel align when the arrow points at a right angle.
        let cross = cos_a * sin_a;
        if is_fill && (-f32::EPSILON..=f32::EPSILON).contains(&cross) {
            for point in &mut p {
                point.x = ((point.x + 0.5) as i32) as f32;
                point.y = ((point.y + 0.5) as i32) as f32;
            }
        }

        if is_fill {
            self.add_convex_poly_filled_aa(&p, col);
        } else {
            self.add_polyline(
                &p,
                col,
                thickness,
                draw_flags::STROKE_INSIDE | draw_flags::AALINE_ENDS,
            );
        }
    }

    /// `ImDrawList::_AddLine`: one segment, selectable cap ends and stroke position.
    pub fn add_line(
        &mut self,
        p1: Vec2,
        p2: Vec2,
        col_in: ImU32,
        thickness_in: f32,
        flags_in: u32,
    ) {
        const A_MASK: u32 = 0xFF00_0000;
        if (col_in & A_MASK) == 0 {
            return;
        }

        let inv_fringe_scale = 1.0 / self.fringe_scale;
        let mut screen_thickness = thickness_in * inv_fringe_scale;
        let mut col = col_in;
        let mut thickness = thickness_in;
        if screen_thickness < 1.0 {
            if screen_thickness < 1.0 / 255.0 {
                return;
            }
            col = alpha_multiply(col, screen_thickness);
            screen_thickness = 1.0;
            thickness = self.fringe_scale;
        }

        let mut flags = flags_in | self.flags;
        let (uv0, uv1, fringe) = self.select_line_texture(screen_thickness, flags);

        let mut dir_x = p2.x - p1.x;
        let mut dir_y = p2.y - p1.y;
        let d2 = dir_x * dir_x + dir_y * dir_y;
        let inv_len = if d2 > 0.0 { rsqrt_precise(d2) } else { 0.0 };
        dir_x *= inv_len;
        dir_y *= inv_len;
        let norm_x = -dir_y;
        let norm_y = dir_x;

        let col_trans = col & !A_MASK;

        let stroke_pos = stroke_pos(flags, draw_flags::STROKE_CENTER);
        if stroke_pos == draw_flags::STROKE_LEGACY {
            flags = (flags | draw_flags::JOIN_MITER) & !draw_flags::AALINE_ENDS;
        }

        let mut thickness0 = (thickness + fringe) * 0.5;
        if stroke_pos == draw_flags::STROKE_OUTSIDE {
            thickness0 = thickness + fringe * 0.5;
        } else if stroke_pos == draw_flags::STROKE_CENTER_BIASED {
            thickness0 = self.center_biased_offset(thickness) + fringe * 0.5;
        } else if stroke_pos == draw_flags::STROKE_INSIDE {
            thickness0 = fringe * 0.5;
        }
        let thickness1 = (thickness + fringe) - thickness0;

        let half_aa = self.fringe_scale * 0.5;
        let use_aa_ends = (flags & (draw_flags::AALINES | draw_flags::AALINE_ENDS))
            == (draw_flags::AALINES | draw_flags::AALINE_ENDS);

        let mut p1_x = p1.x;
        let mut p1_y = p1.y;
        let mut p2_x = p2.x;
        let mut p2_y = p2.y;
        if flags & draw_flags::CAP_SQUARE != 0 {
            let half_thickness = thickness * 0.5;
            p1_x -= dir_x * half_thickness;
            p1_y -= dir_y * half_thickness;
            p2_x += dir_x * half_thickness;
            p2_y += dir_y * half_thickness;
        }

        let mut idx_count = 0u32;
        if !use_aa_ends {
            let base_idx = self.vtx_buffer.len() as u32;
            self.push_vert(
                Vec2 {
                    x: p1_x - norm_x * thickness0,
                    y: p1_y - norm_y * thickness0,
                },
                uv0,
                col,
            );
            self.push_vert(
                Vec2 {
                    x: p1_x + norm_x * thickness1,
                    y: p1_y + norm_y * thickness1,
                },
                uv1,
                col,
            );
            self.push_vert(
                Vec2 {
                    x: p2_x - norm_x * thickness0,
                    y: p2_y - norm_y * thickness0,
                },
                uv0,
                col,
            );
            self.push_vert(
                Vec2 {
                    x: p2_x + norm_x * thickness1,
                    y: p2_y + norm_y * thickness1,
                },
                uv1,
                col,
            );
            for (a, b, c) in [
                (base_idx, base_idx + 2, base_idx + 3),
                (base_idx, base_idx + 3, base_idx + 1),
            ] {
                self.push_tri(a, b, c);
            }
            idx_count += 6;
        } else {
            // Start: two pairs half a fringe apart, the outer one transparent.
            let pa_x = p1_x - dir_x * half_aa;
            let pa_y = p1_y - dir_y * half_aa;
            let pb_x = p1_x + dir_x * half_aa;
            let pb_y = p1_y + dir_y * half_aa;

            let base_idx = self.vtx_buffer.len() as u32;
            self.push_vert(
                Vec2 {
                    x: pa_x - norm_x * thickness0,
                    y: pa_y - norm_y * thickness0,
                },
                uv0,
                col_trans,
            );
            self.push_vert(
                Vec2 {
                    x: pa_x + norm_x * thickness1,
                    y: pa_y + norm_y * thickness1,
                },
                uv1,
                col_trans,
            );

            let mut next_base_idx = self.vtx_buffer.len() as u32;
            self.push_vert(
                Vec2 {
                    x: pb_x - norm_x * thickness0,
                    y: pb_y - norm_y * thickness0,
                },
                uv0,
                col,
            );
            self.push_vert(
                Vec2 {
                    x: pb_x + norm_x * thickness1,
                    y: pb_y + norm_y * thickness1,
                },
                uv1,
                col,
            );

            for (a, b, c) in [
                (base_idx, base_idx + 2, base_idx + 3),
                (base_idx, base_idx + 3, base_idx + 1),
            ] {
                self.push_tri(a, b, c);
            }
            idx_count += 6;
            let mut base_idx = next_base_idx;

            // End.
            let pa_x = p2_x - dir_x * half_aa;
            let pa_y = p2_y - dir_y * half_aa;
            let pb_x = p2_x + dir_x * half_aa;
            let pb_y = p2_y + dir_y * half_aa;

            next_base_idx = self.vtx_buffer.len() as u32;
            self.push_vert(
                Vec2 {
                    x: pa_x - norm_x * thickness0,
                    y: pa_y - norm_y * thickness0,
                },
                uv0,
                col,
            );
            self.push_vert(
                Vec2 {
                    x: pa_x + norm_x * thickness1,
                    y: pa_y + norm_y * thickness1,
                },
                uv1,
                col,
            );
            self.push_vert(
                Vec2 {
                    x: pb_x - norm_x * thickness0,
                    y: pb_y - norm_y * thickness0,
                },
                uv0,
                col_trans,
            );
            self.push_vert(
                Vec2 {
                    x: pb_x + norm_x * thickness1,
                    y: pb_y + norm_y * thickness1,
                },
                uv1,
                col_trans,
            );

            for (a, b, c) in [
                (base_idx, base_idx + 2, base_idx + 3),
                (base_idx, base_idx + 3, base_idx + 1),
            ] {
                self.push_tri(a, b, c);
            }
            idx_count += 6;
            base_idx = next_base_idx;

            for (a, b, c) in [
                (base_idx, base_idx + 2, base_idx + 3),
                (base_idx, base_idx + 3, base_idx + 1),
            ] {
                self.push_tri(a, b, c);
            }
            idx_count += 6;
        }

        if let Some(cmd) = self.cmd_buffer.last_mut() {
            cmd.elem_count += idx_count;
        }
    }

    /// `ImDrawList::_AddPolyline`: every cap, join and stroke-position mode, with the atlas line
    /// textures when the flags ask for anti-aliased strokes.
    ///
    /// `max_inner_offset` is the clamp `AddRect` passes so a thick stroke cannot fold over itself.
    pub fn add_polyline_max(
        &mut self,
        points: &[Vec2],
        col_in: ImU32,
        thickness_in: f32,
        flags_in: u32,
        max_inner_offset: f32,
    ) {
        const A_MASK: u32 = 0xFF00_0000;
        const MITER_ANGLE_LIMIT: f32 = -0.999_961_9;
        const MITER_LIMIT_SQR: f32 = 4.0 * 4.0; // IM_POLYLINE_MITER_LIMIT (4) squared

        let n = points.len();
        if n < 2 || (col_in & A_MASK) == 0 {
            return;
        }

        let inv_fringe_scale = 1.0 / self.fringe_scale;
        let mut screen_thickness = thickness_in * inv_fringe_scale;
        let mut col = col_in;
        let mut thickness = thickness_in;
        if screen_thickness < 1.0 {
            if screen_thickness < 1.0 / 255.0 {
                return;
            }
            col = alpha_multiply(col, screen_thickness);
            screen_thickness = 1.0;
            thickness = self.fringe_scale;
        }

        let mut flags = flags_in | self.flags;

        // One normal and one squared length per segment; the last entry is the closing segment
        // for a closed path, or a copy of the previous normal for an open one.
        let mut normals = vec![Vec2::default(); n];
        let mut sqr_lengths = vec![0.0f32; n];
        for i in 0..n - 1 {
            let dx = points[i + 1].x - points[i].x;
            let dy = points[i + 1].y - points[i].y;
            let d2 = dx * dx + dy * dy;
            let inv = if d2 > 0.0 { rsqrt_precise(d2) } else { 0.0 };
            normals[i] = Vec2 {
                x: -dy * inv,
                y: dx * inv,
            };
            sqr_lengths[i] = d2;
        }
        if flags & draw_flags::CLOSED != 0 {
            let dx = points[0].x - points[n - 1].x;
            let dy = points[0].y - points[n - 1].y;
            let d2 = dx * dx + dy * dy;
            let inv = if d2 > 0.0 { rsqrt_precise(d2) } else { 0.0 };
            normals[n - 1] = Vec2 {
                x: -dy * inv,
                y: dx * inv,
            };
            sqr_lengths[n - 1] = d2;
        } else {
            normals[n - 1] = normals[n - 2];
            sqr_lengths[n - 1] = 0.0;
        }

        let (uv0, mut uv1, fringe) = self.select_line_texture(screen_thickness, flags);
        let col_trans = col & !A_MASK;

        let stroke_pos = stroke_pos(flags, draw_flags::STROKE_CENTER);
        if stroke_pos == draw_flags::STROKE_LEGACY {
            flags = (flags | draw_flags::JOIN_MITER) & !draw_flags::AALINE_ENDS;
        }

        let closed = flags & draw_flags::CLOSED != 0;
        let miters_only = flags & draw_flags::JOIN_MITER != 0;

        let mut thickness0 = (thickness + fringe) * 0.5;
        if stroke_pos == draw_flags::STROKE_OUTSIDE {
            thickness0 = thickness + fringe * 0.5;
        } else if stroke_pos == draw_flags::STROKE_CENTER_BIASED {
            thickness0 = self.center_biased_offset(thickness) + fringe * 0.5;
        } else if stroke_pos == draw_flags::STROKE_INSIDE {
            thickness0 = fringe * 0.5;
        }
        let mut thickness1 = (thickness + fringe) - thickness0;

        // The vertex that sits exactly on the path reads the texture from the stroke ratio.
        let ratio = thickness0 / (thickness0 + thickness1);
        let uv2 = Vec2 {
            x: uv0.x + (uv1.x - uv0.x) * ratio,
            y: uv0.y,
        };

        if thickness1 > max_inner_offset {
            thickness1 = max_inner_offset;
            let s = (thickness0 + thickness1) / (thickness + fringe);
            uv1.x = uv0.x + (uv1.x - uv0.x) * s;
        }

        let half_aa = self.fringe_scale * 0.5;
        let use_aa_ends = (flags & (draw_flags::AALINES | draw_flags::AALINE_ENDS))
            == (draw_flags::AALINES | draw_flags::AALINE_ENDS);

        let first_vtx_offset = self.vtx_buffer.len();
        let mut base_idx;
        let mut idx_count = 0u32;

        let mut p1 = points[0];
        let mut n1 = normals[0];
        let mut len_sqr1 = sqr_lengths[0];

        let mut point_idx = 0usize;
        let mut point_end = n;
        if !closed {
            // Start cap.
            point_idx += 1;
            point_end -= 1;

            let dir_x = n1.y;
            let dir_y = -n1.x;
            if flags & draw_flags::CAP_SQUARE != 0 {
                p1.x -= dir_x * thickness * 0.5;
                p1.y -= dir_y * thickness * 0.5;
            }

            if !use_aa_ends {
                base_idx = self.vtx_buffer.len() as u32;
                self.push_vert(
                    Vec2 {
                        x: p1.x - n1.x * thickness0,
                        y: p1.y - n1.y * thickness0,
                    },
                    uv0,
                    col,
                );
                self.push_vert(
                    Vec2 {
                        x: p1.x + n1.x * thickness1,
                        y: p1.y + n1.y * thickness1,
                    },
                    uv1,
                    col,
                );
            } else {
                let pa = Vec2 {
                    x: p1.x - dir_x * half_aa,
                    y: p1.y - dir_y * half_aa,
                };
                let pb = Vec2 {
                    x: p1.x + dir_x * half_aa,
                    y: p1.y + dir_y * half_aa,
                };

                base_idx = self.vtx_buffer.len() as u32;
                self.push_vert(
                    Vec2 {
                        x: pa.x - n1.x * thickness0,
                        y: pa.y - n1.y * thickness0,
                    },
                    uv0,
                    col_trans,
                );
                self.push_vert(
                    Vec2 {
                        x: pa.x + n1.x * thickness1,
                        y: pa.y + n1.y * thickness1,
                    },
                    uv1,
                    col_trans,
                );

                let next_base_idx = self.vtx_buffer.len() as u32;
                self.push_vert(
                    Vec2 {
                        x: pb.x - n1.x * thickness0,
                        y: pb.y - n1.y * thickness0,
                    },
                    uv0,
                    col,
                );
                self.push_vert(
                    Vec2 {
                        x: pb.x + n1.x * thickness1,
                        y: pb.y + n1.y * thickness1,
                    },
                    uv1,
                    col,
                );

                for (a, b, c) in [
                    (base_idx, base_idx + 2, base_idx + 3),
                    (base_idx, base_idx + 3, base_idx + 1),
                ] {
                    self.push_tri(a, b, c);
                }
                idx_count += 6;
                base_idx = next_base_idx;
            }
        } else {
            // Wrap-around segment: the pair is reserved and patched once the loop is done.
            n1 = normals[n - 1];
            len_sqr1 = sqr_lengths[n - 1];

            base_idx = self.vtx_buffer.len() as u32;
            self.push_vert(Vec2 { x: 0.0, y: 0.0 }, uv0, col);
            self.push_vert(Vec2 { x: 0.0, y: 0.0 }, uv1, col);
        }

        if miters_only {
            while point_idx < point_end {
                let n0 = n1;
                p1 = points[point_idx];
                n1 = normals[point_idx];

                let cos_theta = n0.x * n1.x + n0.y * n1.y;
                let cos_theta_clamped = cos_theta.max(MITER_ANGLE_LIMIT);
                let miter_scale_factor = (1.0 / (1.0 + cos_theta_clamped)).min(1000.0);
                let miter_offset_x = (n0.x + n1.x) * miter_scale_factor;
                let miter_offset_y = (n0.y + n1.y) * miter_scale_factor;

                let next_base_idx = self.vtx_buffer.len() as u32;
                self.push_vert(
                    Vec2 {
                        x: p1.x - miter_offset_x * thickness0,
                        y: p1.y - miter_offset_y * thickness0,
                    },
                    uv0,
                    col,
                );
                self.push_vert(
                    Vec2 {
                        x: p1.x + miter_offset_x * thickness1,
                        y: p1.y + miter_offset_y * thickness1,
                    },
                    uv1,
                    col,
                );

                for (a, b, c) in [
                    (base_idx, base_idx + 2, base_idx + 3),
                    (base_idx, base_idx + 3, base_idx + 1),
                ] {
                    self.push_tri(a, b, c);
                }
                idx_count += 6;
                base_idx = next_base_idx;

                point_idx += 1;
            }
        } else {
            while point_idx < point_end {
                let n0 = n1;
                let len_sqr0 = len_sqr1;
                p1 = points[point_idx];
                n1 = normals[point_idx];
                len_sqr1 = sqr_lengths[point_idx];

                let cos_theta = n0.x * n1.x + n0.y * n1.y;
                let sin_theta = n0.y * n1.x - n0.x * n1.y;

                let cos_theta_clamped = cos_theta.max(MITER_ANGLE_LIMIT);
                let miter_scale_factor = (1.0 / (1.0 + cos_theta_clamped)).min(1000.0);
                let mut miter_offset_x = (n0.x + n1.x) * miter_scale_factor;
                let mut miter_offset_y = (n0.y + n1.y) * miter_scale_factor;

                let miter_distance_sqr =
                    miter_offset_x * miter_offset_x + miter_offset_y * miter_offset_y;

                if miter_distance_sqr > MITER_LIMIT_SQR {
                    // Clipped bevel.
                    let ref_thickness = if sin_theta < 0.0 {
                        thickness1
                    } else {
                        thickness0
                    };
                    let ref_thickness_sqr = ref_thickness * ref_thickness;
                    let limit_sqr = len_sqr0.max(len_sqr1).max(ref_thickness_sqr);
                    let ref_miter_dist_sqr = miter_distance_sqr * ref_thickness_sqr;
                    if ref_miter_dist_sqr > limit_sqr {
                        let scale = (limit_sqr / ref_miter_dist_sqr).sqrt();
                        miter_offset_x *= scale;
                        miter_offset_y *= scale;
                    }

                    let mut bn_x = n0.x + n1.x;
                    let mut bn_y = n0.y + n1.y;
                    let d2 = bn_x * bn_x + bn_y * bn_y;
                    if d2 > 0.0 {
                        let inv = rsqrt_precise(d2);
                        bn_x *= inv;
                        bn_y *= inv;
                    }

                    let side_offset =
                        ((n0.x * bn_x + n0.y * bn_y) - 1.0) / (n0.y * bn_x - n0.x * bn_y);
                    let sd_x = bn_y * side_offset;
                    let sd_y = -bn_x * side_offset;

                    if sin_theta < 0.0 {
                        self.push_vert(
                            Vec2 {
                                x: p1.x - (bn_x + sd_x) * thickness0,
                                y: p1.y - (bn_y + sd_y) * thickness0,
                            },
                            uv0,
                            col,
                        );
                        self.push_vert(p1, uv2, col);
                        let next_base_idx = self.vtx_buffer.len() as u32;
                        self.push_vert(
                            Vec2 {
                                x: p1.x - (bn_x - sd_x) * thickness0,
                                y: p1.y - (bn_y - sd_y) * thickness0,
                            },
                            uv0,
                            col,
                        );
                        self.push_vert(
                            Vec2 {
                                x: p1.x + miter_offset_x * thickness1,
                                y: p1.y + miter_offset_y * thickness1,
                            },
                            uv1,
                            col,
                        );

                        for (a, b, c) in [
                            (base_idx, base_idx + 2, base_idx + 5),
                            (base_idx, base_idx + 5, base_idx + 1),
                            (base_idx + 3, base_idx + 5, base_idx + 2),
                            (base_idx + 3, base_idx + 2, base_idx + 4),
                            (base_idx + 3, base_idx + 4, base_idx + 5),
                        ] {
                            self.push_tri(a, b, c);
                        }
                        idx_count += 15;
                        base_idx = next_base_idx;
                    } else {
                        self.push_vert(
                            Vec2 {
                                x: p1.x + (bn_x + sd_x) * thickness1,
                                y: p1.y + (bn_y + sd_y) * thickness1,
                            },
                            uv1,
                            col,
                        );
                        self.push_vert(p1, uv2, col);
                        let next_base_idx = self.vtx_buffer.len() as u32;
                        self.push_vert(
                            Vec2 {
                                x: p1.x - miter_offset_x * thickness0,
                                y: p1.y - miter_offset_y * thickness0,
                            },
                            uv0,
                            col,
                        );
                        self.push_vert(
                            Vec2 {
                                x: p1.x + (bn_x - sd_x) * thickness1,
                                y: p1.y + (bn_y - sd_y) * thickness1,
                            },
                            uv1,
                            col,
                        );

                        for (a, b, c) in [
                            (base_idx, base_idx + 4, base_idx + 2),
                            (base_idx, base_idx + 2, base_idx + 1),
                            (base_idx + 3, base_idx + 2, base_idx + 4),
                            (base_idx + 3, base_idx + 4, base_idx + 5),
                            (base_idx + 3, base_idx + 5, base_idx + 2),
                        ] {
                            self.push_tri(a, b, c);
                        }
                        idx_count += 15;
                        base_idx = next_base_idx;
                    }
                } else {
                    // Miter.
                    let next_base_idx = self.vtx_buffer.len() as u32;
                    self.push_vert(
                        Vec2 {
                            x: p1.x - miter_offset_x * thickness0,
                            y: p1.y - miter_offset_y * thickness0,
                        },
                        uv0,
                        col,
                    );
                    self.push_vert(
                        Vec2 {
                            x: p1.x + miter_offset_x * thickness1,
                            y: p1.y + miter_offset_y * thickness1,
                        },
                        uv1,
                        col,
                    );

                    for (a, b, c) in [
                        (base_idx, base_idx + 2, base_idx + 3),
                        (base_idx, base_idx + 3, base_idx + 1),
                    ] {
                        self.push_tri(a, b, c);
                    }
                    idx_count += 6;
                    base_idx = next_base_idx;
                }

                point_idx += 1;
            }
        }

        if !closed {
            // End cap.
            p1 = points[n - 1];
            n1 = normals[n - 1];

            let dir_x = n1.y;
            let dir_y = -n1.x;
            if flags & draw_flags::CAP_SQUARE != 0 {
                p1.x += dir_x * thickness * 0.5;
                p1.y += dir_y * thickness * 0.5;
            }

            if !use_aa_ends {
                self.push_vert(
                    Vec2 {
                        x: p1.x - n1.x * thickness0,
                        y: p1.y - n1.y * thickness0,
                    },
                    uv0,
                    col,
                );
                self.push_vert(
                    Vec2 {
                        x: p1.x + n1.x * thickness1,
                        y: p1.y + n1.y * thickness1,
                    },
                    uv1,
                    col,
                );

                for (a, b, c) in [
                    (base_idx, base_idx + 2, base_idx + 3),
                    (base_idx, base_idx + 3, base_idx + 1),
                ] {
                    self.push_tri(a, b, c);
                }
                idx_count += 6;
            } else {
                let pa = Vec2 {
                    x: p1.x - dir_x * half_aa,
                    y: p1.y - dir_y * half_aa,
                };
                let pb = Vec2 {
                    x: p1.x + dir_x * half_aa,
                    y: p1.y + dir_y * half_aa,
                };

                let next_base_idx = self.vtx_buffer.len() as u32;
                self.push_vert(
                    Vec2 {
                        x: pa.x - n1.x * thickness0,
                        y: pa.y - n1.y * thickness0,
                    },
                    uv0,
                    col,
                );
                self.push_vert(
                    Vec2 {
                        x: pa.x + n1.x * thickness1,
                        y: pa.y + n1.y * thickness1,
                    },
                    uv1,
                    col,
                );
                self.push_vert(
                    Vec2 {
                        x: pb.x - n1.x * thickness0,
                        y: pb.y - n1.y * thickness0,
                    },
                    uv0,
                    col_trans,
                );
                self.push_vert(
                    Vec2 {
                        x: pb.x + n1.x * thickness1,
                        y: pb.y + n1.y * thickness1,
                    },
                    uv1,
                    col_trans,
                );

                for (a, b, c) in [
                    (base_idx, base_idx + 2, base_idx + 3),
                    (base_idx, base_idx + 3, base_idx + 1),
                ] {
                    self.push_tri(a, b, c);
                }
                idx_count += 6;
                base_idx = next_base_idx;

                // AA cap.
                for (a, b, c) in [
                    (base_idx, base_idx + 2, base_idx + 3),
                    (base_idx, base_idx + 3, base_idx + 1),
                ] {
                    self.push_tri(a, b, c);
                }
                idx_count += 6;
            }
        } else {
            // Connect the path: the reserved pair takes the last pair, as the reference copies.
            let last = self.vtx_buffer[base_idx as usize];
            self.vtx_buffer[first_vtx_offset] = last;
            self.vtx_buffer[first_vtx_offset + 1] = self.vtx_buffer[base_idx as usize + 1];
        }

        if let Some(cmd) = self.cmd_buffer.last_mut() {
            cmd.elem_count += idx_count;
        }
    }

    /// `ImDrawList::_CalculateCenterBiasedOffset`: the outside offset of the center-biased stroke.
    fn center_biased_offset(&self, thickness: f32) -> f32 {
        let screen_thickness = thickness / self.fringe_scale;
        let s_thickness = screen_thickness as i32;
        ((s_thickness / 2) as f32
            + (s_thickness & 1) as f32 * (screen_thickness - s_thickness as f32))
            * self.fringe_scale
    }

    /// `ImDrawList::_SelectLineTexture`: the atlas row a stroke samples, and the fringe it adds.
    ///
    /// Returns `(uv0, uv1, fringe)`. Without `ImDrawFlags_AALines` the reference samples the white
    /// pixel and adds no fringe, which is the untextured path.
    fn select_line_texture(&self, screen_thickness: f32, flags: u32) -> (Vec2, Vec2, f32) {
        const WIDTH_MAX: i32 = 32;
        const DETAILED_WIDTH: f32 = 4.0;
        const SAMPLE_COUNT: f32 = 4.0;

        if flags & draw_flags::AALINES == 0 {
            return (self.white_uv, self.white_uv, 0.0);
        }

        let texture_idx: i32 = if screen_thickness <= DETAILED_WIDTH {
            ((screen_thickness - 1.0) * SAMPLE_COUNT + 0.5) as i32 + (WIDTH_MAX + 1)
        } else if screen_thickness < (WIDTH_MAX as f32) - 1.5 {
            (screen_thickness + 0.5) as i32
        } else {
            WIDTH_MAX - 1
        };
        let entry = *self
            .tex_uv_lines
            .get(texture_idx as usize)
            .unwrap_or_else(|| {
                panic!(
                    "the atlas line-texture table is not pinned (index {texture_idx}); \
                 a stroked primitive with anti-aliased lines needs it, see \
                 Context::set_line_uv_lines"
                )
            });
        (
            Vec2 {
                x: entry[0],
                y: entry[2],
            },
            Vec2 {
                x: entry[1],
                y: entry[2],
            },
            self.fringe_scale * screen_thickness * entry[3],
        )
    }

    fn push_vert(&mut self, pos: Vec2, uv: Vec2, col: ImU32) {
        self.vtx_buffer.push(DrawVert { pos, uv, col });
    }

    fn push_tri(&mut self, a: u32, b: u32, c: u32) {
        for index in [a, b, c] {
            self.idx_buffer.push(index as ImDrawIdx);
        }
    }
    /// `ImDrawList::AddRect` for a pixel-aligned rectangle with the inside stroke: a two-ring frame
    /// (outer and inner corners) of four quads.
    pub fn add_aligned_rect_border(&mut self, min: Vec2, max: Vec2, col: ImU32, thickness: f32) {
        let uv = self.white_uv;
        let base = self.vtx_buffer.len() as u32;
        let t = thickness;
        let corners = [
            (
                min,
                Vec2 {
                    x: min.x + t,
                    y: min.y + t,
                },
            ),
            (
                Vec2 { x: max.x, y: min.y },
                Vec2 {
                    x: max.x - t,
                    y: min.y + t,
                },
            ),
            (
                max,
                Vec2 {
                    x: max.x - t,
                    y: max.y - t,
                },
            ),
            (
                Vec2 { x: min.x, y: max.y },
                Vec2 {
                    x: min.x + t,
                    y: max.y - t,
                },
            ),
        ];
        for (outer, inner) in corners {
            self.vtx_buffer.push(DrawVert {
                pos: outer,
                uv,
                col,
            });
            self.vtx_buffer.push(DrawVert {
                pos: inner,
                uv,
                col,
            });
        }
        let mut idx_count = 0u32;
        for edge in 0..4u32 {
            let a = base + edge * 2;
            let b = a + 1;
            let c = base + ((edge + 1) % 4) * 2 + 1;
            let d = base + ((edge + 1) % 4) * 2;
            // Same winding as the closed inside stroke: (outer, next inner, inner), then
            // (outer, next outer, next inner).
            for vertex in [a, c, b, a, d, c] {
                self.idx_buffer.push(vertex as ImDrawIdx);
            }
            idx_count += 6;
        }
        if let Some(cmd) = self.cmd_buffer.last_mut() {
            cmd.elem_count += idx_count;
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

    /// `AddLineH` with `StrokeCenterBiased`, on the pixel-aligned path the reference takes when the
    /// coordinates and the thickness are whole pixels: a `PrimRect` whose top edge is shifted up by
    /// `_CalculateCenterBiasedOffset`, so both sides of the line land on pixel boundaries.
    pub fn add_line_h_center_biased(
        &mut self,
        min_x: f32,
        max_x: f32,
        y: f32,
        col: ImU32,
        thickness: f32,
    ) {
        let screen_thickness = thickness / self.fringe_scale;
        let whole = screen_thickness as i32;
        let offset = (whole / 2) as f32 + (whole & 1) as f32 * (screen_thickness - whole as f32);
        let top_y = y - offset * self.fringe_scale;
        self.prim_rect(
            Vec2 { x: min_x, y: top_y },
            Vec2 {
                x: max_x,
                y: top_y + thickness,
            },
            col,
        );
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

    /// `AddLineV` with one pixel and aligned coordinates: the reference's fast path, a `PrimRect`
    /// from `(x, min_y)` to `(x + 1, max_y)`.
    pub fn add_line_v(&mut self, x: f32, min_y: f32, max_y: f32, col: ImU32, thickness: f32) {
        self.prim_rect(
            Vec2 { x, y: min_y },
            Vec2 {
                x: x + thickness,
                y: max_y,
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
/// `PathBezierCubicCurveToCasteljau`: recursive subdivision until the flatness test passes. Pure
/// arithmetic, so it is portable bit for bit.
fn bezier_cubic_casteljau(
    path: &mut Vec<Vec2>,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    p4: Vec2,
    max_error_sqr: f32,
    level: i32,
) {
    let dx = p4.x - p1.x;
    let dy = p4.y - p1.y;
    let mut d2 = (p2.x - p4.x) * dy - (p2.y - p4.y) * dx;
    let mut d3 = (p3.x - p4.x) * dy - (p3.y - p4.y) * dx;
    if d2 < 0.0 {
        d2 = -d2;
    }
    if d3 < 0.0 {
        d3 = -d3;
    }
    let sum = d2 + d3;
    if sum * sum < max_error_sqr * (dx * dx + dy * dy) {
        path.push(p4);
        return;
    }
    if level < 10 {
        let p12 = Vec2 {
            x: (p1.x + p2.x) * 0.5,
            y: (p1.y + p2.y) * 0.5,
        };
        let p23 = Vec2 {
            x: (p2.x + p3.x) * 0.5,
            y: (p2.y + p3.y) * 0.5,
        };
        let p34 = Vec2 {
            x: (p3.x + p4.x) * 0.5,
            y: (p3.y + p4.y) * 0.5,
        };
        let p123 = Vec2 {
            x: (p12.x + p23.x) * 0.5,
            y: (p12.y + p23.y) * 0.5,
        };
        let p234 = Vec2 {
            x: (p23.x + p34.x) * 0.5,
            y: (p23.y + p34.y) * 0.5,
        };
        let p1234 = Vec2 {
            x: (p123.x + p234.x) * 0.5,
            y: (p123.y + p234.y) * 0.5,
        };
        bezier_cubic_casteljau(path, p1, p12, p123, p1234, max_error_sqr, level + 1);
        bezier_cubic_casteljau(path, p1234, p234, p34, p4, max_error_sqr, level + 1);
    }
}

/// `ImDrawList::PathBezierCubicCurveTo` with automatic tessellation: appends the curve points to
/// `path`, whose last element is the curve start.
pub fn path_bezier_cubic_to(
    path: &mut Vec<Vec2>,
    p2: Vec2,
    p3: Vec2,
    p4: Vec2,
    curve_max_error: f32,
    fringe_scale: f32,
) {
    let Some(&p1) = path.last() else {
        return;
    };
    let error = curve_max_error * fringe_scale;
    let max_error_sqr = error * error;
    bezier_cubic_casteljau(path, p1, p2, p3, p4, max_error_sqr, 0);
}

/// `IM_DRAWLIST_ARCFAST_SAMPLE_MAX`: samples around a full circle in the fast arc table.
const ARC_SAMPLE_MAX: i32 = 48;

/// `IM_DRAWLIST_CIRCLE_AUTO_SEGMENT_MIN` / `_MAX`: what `AddCircle*` clamps a segment count to.
const CIRCLE_SEGMENT_MIN: i32 = 4;
const CIRCLE_SEGMENT_MAX: i32 = 512;

/// `ImGuiStyle::CircleTessellationMaxError` for the default style.
const CIRCLE_TESS_MAX_ERROR: f32 = 0.30;

/// `ImDrawListSharedData::CircleSegmentCounts`: the auto segment count cached for integer radii
/// 0..=63. The reference fills it at start-up with `IM_DRAWLIST_CIRCLE_AUTO_SEGMENT_CALC`, whose
/// `acos` is not portable in its last bits, and a golden has to come out the same on every machine.
/// Radii past the table go through [`circle_segment_count`], the same formula, unpinned.
const CIRCLE_SEGMENT_COUNTS: [u8; 64] = [
    48, 4, 6, 8, 10, 10, 10, 12, 12, 14, 14, 14, 16, 16, 16, 16, 18, 18, 18, 18, 20, 20, 20, 20,
    20, 22, 22, 22, 22, 22, 24, 24, 24, 24, 24, 24, 26, 26, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28,
    30, 30, 30, 30, 30, 30, 30, 32, 32, 32, 32, 32, 32, 32, 32, 34,
];

/// `ImDrawListSharedData::ArcFastRadiusCutoff` = `CALC_R(48, 0.30)`, pinned as bits (140.1176) for
/// the same reason as the table: the formula needs `cos`.
const ARC_FAST_RADIUS_CUTOFF: f32 = f32::from_bits(0x430c_1e1c);

/// The reference writes `1.41421356f` literally for the smallest radius whose edge fringe does not
/// overlap itself. That is not an exact `sqrt(2)` and the digits past the f32 mantissa are kept as
/// written, so the literal stays instead of a "better" constant.
#[allow(clippy::approx_constant, clippy::excessive_precision)]
const REF_1_41421356: f32 = 1.414_213_56;

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
///
/// It uses `acos`, so its last bits are not portable: the reference caches it for radii 0..=63 in
/// `CircleSegmentCounts` (pinned below) and only calls this for the radii past the table, which no
/// golden covers yet.
pub fn circle_segment_count(radius: f32, max_error: f32) -> i32 {
    if radius <= 0.0 {
        return ARC_SAMPLE_MAX;
    }
    let error = max_error.min(radius);
    let angle = (1.0f32 - error / radius).acos();
    let raw = (core::f32::consts::PI / angle).ceil() as i32;
    let even = ((raw + 1) / 2) * 2;
    even.clamp(CIRCLE_SEGMENT_MIN, CIRCLE_SEGMENT_MAX)
}

/// `CalcCornerArcStepAndCount`: the sampling step for a 90 degree arc, and how many steps fill it.
/// The step has to divide the quarter circle exactly, which is what the re-derivation at the end
/// guarantees.
fn corner_arc_step_and_count(segment_count: i32) -> (i32, i32) {
    const QUARTER: i32 = ARC_SAMPLE_MAX / 4;
    let mut arc_step = (ARC_SAMPLE_MAX / segment_count.max(1)).clamp(1, QUARTER);
    let arc_step_count = QUARTER / arc_step;
    if arc_step * arc_step_count != QUARTER {
        arc_step = QUARTER / arc_step_count.max(1);
    }
    (arc_step, arc_step_count)
}

/// The sampling step within a quarter circle for a given segment count.
pub fn arc_step_for_segments(segment_count: i32) -> i32 {
    corner_arc_step_and_count(segment_count).0
}

/// The sampling step the reference uses for a corner of this radius.
pub fn arc_step_for_radius(radius: f32) -> i32 {
    arc_step_for_segments(circle_segment_count(radius, CIRCLE_TESS_MAX_ERROR))
}

/// `ImDrawList::_PathArcToFastEx`, the body after the shared data has been consulted: it walks the
/// fast arc table from `a_min_sample` to `a_max_sample` and pushes one point per sample.
///
/// `a_step` must already be resolved and clamped. When the range is not a multiple of the step the
/// reference distributes the remainder: it shortens the first step and appends one last point at the
/// exact maximum sample.
fn push_arc_samples(
    out: &mut Vec<Vec2>,
    center: Vec2,
    radius: f32,
    a_min_sample: i32,
    a_max_sample: i32,
    mut a_step: i32,
) {
    let sample_range = (a_max_sample - a_min_sample).abs();
    let next_step = a_step;
    let mut extra_max_sample = false;
    if a_step > 1 {
        let overstep = sample_range % a_step;
        if overstep > 0 {
            extra_max_sample = true;
            if sample_range > 0 {
                a_step -= (a_step - overstep) / 2;
            }
        }
    }

    let table = arc_table();
    let mut sample_index = a_min_sample;
    if !(0..ARC_SAMPLE_MAX).contains(&sample_index) {
        sample_index %= ARC_SAMPLE_MAX;
        if sample_index < 0 {
            sample_index += ARC_SAMPLE_MAX;
        }
    }

    let mut step = a_step;
    let mut a = a_min_sample;
    if a_max_sample >= a_min_sample {
        while a <= a_max_sample {
            if sample_index >= ARC_SAMPLE_MAX {
                sample_index -= ARC_SAMPLE_MAX;
            }
            let sample = table[sample_index as usize];
            out.push(Vec2 {
                x: center.x + sample.x * radius,
                y: center.y + sample.y * radius,
            });
            a += step;
            sample_index += step;
            step = next_step;
        }
    } else {
        while a >= a_max_sample {
            if sample_index < 0 {
                sample_index += ARC_SAMPLE_MAX;
            }
            let sample = table[sample_index as usize];
            out.push(Vec2 {
                x: center.x + sample.x * radius,
                y: center.y + sample.y * radius,
            });
            a -= step;
            sample_index -= step;
            step = next_step;
        }
    }

    if extra_max_sample {
        let mut index = a_max_sample % ARC_SAMPLE_MAX;
        if index < 0 {
            index += ARC_SAMPLE_MAX;
        }
        let sample = table[index as usize];
        out.push(Vec2 {
            x: center.x + sample.x * radius,
            y: center.y + sample.y * radius,
        });
    }
}

/// `ImDrawList::PathArcToFast`, which the rounded corners use: the two ends come as twelfths of a
/// circle and the step was already chosen for the radius.
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
    let step = arc_step.clamp(1, ARC_SAMPLE_MAX / 4);
    push_arc_samples(out, center, radius, min_sample, max_sample, step);
}

/// `ApproxApothem`: the cosine of half the angle of a regular polygon, to 0.001 %, which the
/// reference uses to clamp the inner stroke of a circle so it does not shoot past the edge.
fn approx_apothem(num_segments: i32) -> f32 {
    let u = 1.0 / num_segments as f32;
    ((4.058_712 * u * u) - 4.934_802) * u * u + 1.0
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
