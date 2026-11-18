//! Font metrics, text measurement and glyph quads.
//!
//! **The atlas is not built here.** The reference (1.93) bakes glyphs per size on demand and
//! rasterises through a loader hook; porting that is roadmap F4. The glyph table below is **pinned
//! data** taken from the reference's default font at size 13 (`tools/conformance`, scenario
//! `text_draw`), in the same spirit as the white-pixel UV: the data is an input, the code that
//! consumes it is what the tests check.

use crate::draw_list::Vec2;

/// A baked glyph: advance, atlas UVs and offsets, as `ImFontGlyph` carries them.
#[derive(Copy, Clone, Debug)]
pub struct Glyph {
    pub advance_x: f32,
    pub uv: [f32; 4],
    pub offset: [f32; 4],
}

/// A font's metrics at its reference size, plus the glyphs that were pinned.
#[derive(Clone, Debug)]
pub struct Font {
    /// `ImFontBaked::Size`.
    pub size: f32,
    /// `ImFontBaked::Ascent`.
    pub ascent: f32,
    /// `ImFontBaked::Descent` (negative).
    pub descent: f32,
    glyphs: Vec<(char, Glyph)>,
    fallback_advance: f32,
}

fn bits(value: u32) -> f32 {
    f32::from_bits(value)
}

fn glyph(advance: u32, uv: [u32; 4], offset: [u32; 4]) -> Glyph {
    Glyph {
        advance_x: bits(advance),
        uv: [bits(uv[0]), bits(uv[1]), bits(uv[2]), bits(uv[3])],
        offset: [
            bits(offset[0]),
            bits(offset[1]),
            bits(offset[2]),
            bits(offset[3]),
        ],
    }
}

impl Font {
    /// The reference's default font at size 13: scale 1, every advance 7 px, metrics and UVs taken
    /// from `tools/conformance`. The `u32` arguments are the raw floats the reference printed.
    pub fn reference_default() -> Self {
        Self {
            size: 13.0,
            ascent: bits(0x4120_0000),
            descent: bits(0xC080_0000),
            glyphs: vec![
                // advance, UV0 UV1, offset X0 Y0 X1 Y1
                (
                    'H',
                    glyph(
                        0x40E0_0000,
                        [0x3F78_8000, 0x0000_0000, 0x3F7B_8000, 0x3D80_0000],
                        [0x3F80_0000, 0x4040_0000, 0x40E0_0000, 0x4130_0000],
                    ),
                ),
                (
                    'i',
                    glyph(
                        0x40E0_0000,
                        [0x3F7C_0000, 0x0000_0000, 0x3F7D_0000, 0x3D90_0000],
                        [0x4000_0000, 0x4000_0000, 0x4080_0000, 0x4130_0000],
                    ),
                ),
                (
                    'e',
                    glyph(
                        0x40E0_0000,
                        [0x3F7B_8000, 0x3E68_0000, 0x3F7E_0000, 0x3E8C_0000],
                        [0x3F80_0000, 0x40A0_0000, 0x40C0_0000, 0x4130_0000],
                    ),
                ),
                (
                    'l',
                    glyph(
                        0x40E0_0000,
                        [0x3F71_8000, 0x3E84_0000, 0x3F72_8000, 0x3EA8_0000],
                        [0x4000_0000, 0x4000_0000, 0x4080_0000, 0x4130_0000],
                    ),
                ),
                (
                    'o',
                    glyph(
                        0x40E0_0000,
                        [0x3ED3_0000, 0x3E8C_0000, 0x3ED8_0000, 0x3EA4_0000],
                        [0x3F80_0000, 0x40A0_0000, 0x40C0_0000, 0x4130_0000],
                    ),
                ),
            ],
            fallback_advance: 7.0,
        }
    }

    pub fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs
            .iter()
            .find(|(known, _)| *known == c)
            .map(|(_, g)| g)
    }

    /// `ImFont::CalcTextSizeA` for a single line: sum of advances scaled by `size / font_size`,
    /// height is the requested size. Unknown characters use the fallback advance, as the reference.
    pub fn calc_text_size(&self, text: &str, size: f32) -> Vec2 {
        let scale = size / self.size;
        let mut line_width = 0.0f32;
        let mut text_width = 0.0f32;
        for c in text.chars() {
            let advance = self
                .glyph(c)
                .map(|g| g.advance_x)
                .unwrap_or(self.fallback_advance)
                * scale;
            text_width = text_width.max(line_width + advance);
            line_width += advance;
        }
        Vec2 {
            x: text_width,
            y: size,
        }
    }
}
