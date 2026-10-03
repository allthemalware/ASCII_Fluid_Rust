//! Renders the fluid field as ASCII glyphs directly into the window's
//! `u32` pixel buffer using the embedded 8x8 bitmap font. No per-character
//! terminal escapes, no GPU text layout — just blits.

use crate::fluid::Fluid;
use font8x8::legacy::{BASIC_LEGACY, BLOCK_LEGACY, BOX_LEGACY, HIRAGANA_LEGACY, LATIN_LEGACY};

#[derive(Clone, Copy, PartialEq)]
pub enum Field {
    Dye,
    Speed,
    Vort,
}

pub struct View<'a> {
    pub lut: &'a [u32; 256],
    pub ramp: &'a [char],
    pub field: Field,
    pub invert: bool,
    pub gamma: f32,
    pub spin: bool, // signed color, only meaningful for Field::Vort
    pub cell: usize,
    pub cols: usize,
    pub rows: usize,
    pub off_x: usize,
    pub off_y: usize,
    pub inv_norm: f32, // 1.0 / running peak (auto-exposure)
}

/// Hand-drawn 8x8 bitmaps for ramp glyphs font8x8 lacks. Bit order matches
/// `blit_glyph`: LSB = leftmost pixel.
const CUSTOM_GLYPHS: &[(char, [u8; 8])] = &[
    // dots
    ('\u{2022}', [0x00, 0x00, 0x18, 0x3C, 0x3C, 0x18, 0x00, 0x00]), // • bullet
    ('\u{25cf}', [0x00, 0x3C, 0x7E, 0x7E, 0x7E, 0x7E, 0x3C, 0x00]), // ● black circle
    // arrows
    ('\u{2191}', [0x18, 0x3C, 0x7E, 0x18, 0x18, 0x18, 0x18, 0x00]), // ↑
    ('\u{2192}', [0x00, 0x18, 0x30, 0x7F, 0x7F, 0x30, 0x18, 0x00]), // →
    ('\u{2193}', [0x00, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x3C, 0x18]), // ↓
    ('\u{2190}', [0x00, 0x18, 0x0C, 0xFE, 0xFE, 0x0C, 0x18, 0x00]), // ←
    ('\u{2194}', [0x00, 0x00, 0x42, 0xFF, 0xFF, 0x42, 0x00, 0x00]), // ↔
    ('\u{2195}', [0x18, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x18]), // ↕
    ('\u{21d1}', [0x18, 0x3C, 0x66, 0x24, 0x24, 0x24, 0x24, 0x00]), // ⇑
    ('\u{21d2}', [0x00, 0x10, 0x3F, 0x40, 0x40, 0x3F, 0x10, 0x00]), // ⇒
    ('\u{21d3}', [0x00, 0x24, 0x24, 0x24, 0x24, 0x66, 0x3C, 0x18]), // ⇓
    ('\u{21d0}', [0x00, 0x08, 0xFC, 0x02, 0x02, 0xFC, 0x08, 0x00]), // ⇐
    ('\u{27a4}', [0x00, 0x03, 0x0F, 0x3F, 0x3F, 0x0F, 0x03, 0x00]), // ➤
    // stars
    ('\u{2727}', [0x00, 0x08, 0x14, 0x22, 0x14, 0x08, 0x00, 0x00]), // ✧ hollow 4-point
    ('\u{2726}', [0x00, 0x08, 0x1C, 0x3E, 0x1C, 0x08, 0x00, 0x00]), // ✦ filled 4-point
    ('\u{2605}', [0x08, 0x1C, 0x7F, 0x3E, 0x1C, 0x36, 0x22, 0x00]), // ★ 5-point
    ('\u{2736}', [0x00, 0x08, 0x2A, 0x1C, 0x2A, 0x08, 0x00, 0x00]), // ✶ 6-point
    ('\u{2737}', [0x00, 0x08, 0x2A, 0x1C, 0x7F, 0x1C, 0x2A, 0x08]), // ✷ 8-point
    ('\u{274b}', [0x18, 0x5A, 0x3C, 0x7E, 0x3C, 0x5A, 0x18, 0x00]), // ❋ heavy asterisk
];

/// Synthesize a braille-pattern glyph (U+2800..U+28FF): codepoint bits 0..7
/// are dots 1..8, laid out as two columns of four 2x2-pixel dots.
fn braille(u: u32) -> [u8; 8] {
    // bit k -> (column, row) per the Unicode dot numbering
    const POS: [(u8, u8); 8] = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (0, 3), (1, 3)];
    let bits = u - 0x2800;
    let mut g = [0u8; 8];
    for (k, &(col, row)) in POS.iter().enumerate() {
        if bits & (1 << k) != 0 {
            let m = if col == 0 { 0b0000_0110 } else { 0b0110_0000 };
            g[row as usize * 2] |= m;
            g[row as usize * 2 + 1] |= m;
        }
    }
    g
}

/// Look up an 8x8 glyph: ASCII + latin-1, box/block drawing, braille
/// (synthesized), hiragana, and a small custom table for the rest.
pub fn glyph(c: char) -> Option<[u8; 8]> {
    let u = c as u32;
    match u {
        0..=0x7F => Some(BASIC_LEGACY[u as usize]),
        0xA0..=0xFF => Some(LATIN_LEGACY[(u - 0xA0) as usize]),
        0x2500..=0x257F => Some(BOX_LEGACY[(u - 0x2500) as usize]),
        0x2580..=0x259F => Some(BLOCK_LEGACY[(u - 0x2580) as usize]),
        0x2800..=0x28FF => Some(braille(u)),
        0x3040..=0x309F => Some(HIRAGANA_LEGACY[(u - 0x3040) as usize]),
        _ => CUSTOM_GLYPHS.iter().find(|(ch, _)| *ch == c).map(|(_, g)| *g),
    }
}

/// Blit one glyph, nearest-neighbour scaled from 8x8 into a `cell`x`cell` box.
#[inline]
fn blit_glyph(buf: &mut [u32], width: usize, height: usize, x0: usize, y0: usize, cell: usize, g: &[u8; 8], color: u32) {
    for py in 0..cell {
        let yy = y0 + py;
        if yy >= height {
            break;
        }
        let row = g[(py * 8) / cell];
        if row == 0 {
            continue;
        }
        let base = yy * width;
        for px in 0..cell {
            let gx = (px * 8) / cell;
            if row & (1 << gx) != 0 {
                let xx = x0 + px;
                if xx < width {
                    buf[base + xx] = color;
                }
            }
        }
    }
}

pub fn fill(buf: &mut [u32], color: u32) {
    buf.iter_mut().for_each(|p| *p = color);
}

pub fn fill_rect(buf: &mut [u32], width: usize, height: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
    for yy in y..(y + h).min(height) {
        let base = yy * width;
        for xx in x..(x + w).min(width) {
            buf[base + xx] = color;
        }
    }
}

/// Draw a line of text at a fixed glyph size (HUD / labels).
pub fn blit_text(buf: &mut [u32], width: usize, height: usize, mut x: usize, y: usize, scale: usize, text: &str, color: u32) {
    for c in text.chars() {
        if c != ' ' {
            if let Some(g) = glyph(c) {
                blit_glyph(buf, width, height, x, y, scale, &g, color);
            }
        }
        x += scale;
    }
}

/// Compute the peak of the currently-viewed field over the interior grid.
pub fn scan_max(fluid: &Fluid, field: Field, cols: usize, rows: usize) -> f32 {
    let mut m = 1e-4f32;
    for j in 1..=rows {
        for i in 1..=cols {
            let s = match field {
                Field::Dye => fluid.dye(i, j),
                Field::Speed => fluid.speed(i, j),
                Field::Vort => fluid.vorticity(i, j).abs(),
            };
            if s > m {
                m = s;
            }
        }
    }
    m
}

/// Render the field into `buf`. Background must already be cleared by caller.
pub fn draw_field(buf: &mut [u32], width: usize, height: usize, fluid: &Fluid, view: &View) {
    let rmax = view.ramp.len() - 1;
    let use_spin = view.spin && view.field == Field::Vort;
    let inv = view.inv_norm;
    let cell = view.cell;

    for j in 1..=view.rows {
        let y0 = view.off_y + (j - 1) * cell;
        for i in 1..=view.cols {
            let (raw, signed) = match view.field {
                Field::Dye => (fluid.dye(i, j), 0.0),
                Field::Speed => (fluid.speed(i, j), 0.0),
                Field::Vort => {
                    let s = fluid.vorticity(i, j);
                    (s.abs(), s)
                }
            };

            let mut v = (raw * inv).clamp(0.0, 1.0).powf(view.gamma);
            let vv = if view.invert { 1.0 - v } else { v };

            let ci = ((vv * rmax as f32) as usize).min(rmax);
            let ch = view.ramp[ci];
            if ch == ' ' {
                continue;
            }

            let li = if use_spin {
                let s = (signed * inv).clamp(-1.0, 1.0);
                let mag = s.abs().powf(view.gamma) * s.signum();
                ((mag * 0.5 + 0.5) * 255.0) as usize
            } else {
                v = vv; // keep brightness consistent with the chosen char
                (v * 255.0) as usize
            }
            .min(255);

            let color = view.lut[li];
            let x0 = view.off_x + (i - 1) * cell;
            if let Some(g) = glyph(ch) {
                blit_glyph(buf, width, height, x0, y0, cell, &g, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braille_dot_layout() {
        // ⠁ = dot 1 only: a 2x2 dot in the top-left cell, nothing else.
        let g = glyph('\u{2801}').unwrap();
        assert_eq!(g[0], 0b0000_0110);
        assert_eq!(g[1], 0b0000_0110);
        assert!(g[2..].iter().all(|&r| r == 0));

        // ⣿ = all eight dots: both columns filled on every dot row.
        let g = glyph('\u{28FF}').unwrap();
        assert!(g.iter().all(|&r| r == 0b0110_0110));

        // ⠈ = dot 4 only: top-right.
        let g = glyph('\u{2808}').unwrap();
        assert_eq!(g[0], 0b0110_0000);
        assert!(g[2..].iter().all(|&r| r == 0));
    }

    #[test]
    fn every_ramp_char_has_a_glyph() {
        for ramp in crate::palette::RAMPS {
            for ch in ramp.chars.chars().skip(1) {
                let g = glyph(ch);
                assert!(g.is_some(), "ramp '{}': no glyph for {ch:?}", ramp.name);
                assert!(
                    g.unwrap().iter().any(|&r| r != 0),
                    "ramp '{}': blank glyph for {ch:?}",
                    ramp.name
                );
            }
        }
    }
}
