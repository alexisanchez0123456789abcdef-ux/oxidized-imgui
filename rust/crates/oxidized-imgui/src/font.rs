//! Font metrics and text measurement.
//!
//! This is deliberately partial: **the atlas is not built here**. The reference (1.93) uses a
//! dynamic font system — glyphs are baked per size on demand, and the rasteriser is a loader hook.
//! Porting that is roadmap F4, not this step.
//!
//! What exists: the default font's metrics and the per-glyph advances, taken from the reference and
//! pinned as inputs, plus `CalcTextSize` computed from them. The advance table is data, like the
//! white-pixel UV; the text measurement is the part under test.

use crate::draw_list::Vec2;

/// A glyph entry: only the advance is needed to measure text. Offsets and UVs arrive with text
/// rendering, which needs the atlas.
#[derive(Copy, Clone, Debug)]
pub struct Glyph {
    pub advance_x: f32,
}

/// A font's metrics at its reference size.
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

impl Font {
    /// The reference's default font at size 13, as printed by `tools/conformance`:
    /// size 13, ascent 10, descent -4, every glyph advance 7 (it is monospaced).
    ///
    /// Pinned data, not a computed atlas: repinning it means re-running the driver.
    pub fn reference_default() -> Self {
        let advance = 7.0;
        Self {
            size: 13.0,
            ascent: 10.0,
            descent: -4.0,
            glyphs: vec![
                ('H', Glyph { advance_x: advance }),
                ('i', Glyph { advance_x: advance }),
                ('W', Glyph { advance_x: advance }),
                ('e', Glyph { advance_x: advance }),
                ('l', Glyph { advance_x: advance }),
                ('o', Glyph { advance_x: advance }),
                (' ', Glyph { advance_x: advance }),
            ],
            fallback_advance: advance,
        }
    }

    pub fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs
            .iter()
            .find(|(known, _)| *known == c)
            .map(|(_, g)| g)
    }

    /// `ImFont::CalcTextSizeA` for a single line: sum of advances scaled by `size / font_size`,
    /// height is the requested size. Unknown characters use the fallback glyph, as the reference.
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
