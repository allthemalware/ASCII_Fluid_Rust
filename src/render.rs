//! Renders the fluid field as ASCII glyphs directly into the window's
//! `u32` pixel buffer using the embedded 8x8 bitmap font. No per-character
//! terminal escapes, no GPU text layout — just blits.

use crate::fluid::Fluid;
use font8x8::legacy::{BASIC_LEGACY, BLOCK_LEGACY};

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

/// Look up an 8x8 glyph for printable ASCII and the shade-block range.
fn glyph(c: char) -> Option<[u8; 8]> {
    let u = c as u32;
    if u < 128 {
        Some(BASIC_LEGACY[u as usize])
    } else if (0x2580..=0x259F).contains(&u) {
        Some(BLOCK_LEGACY[(u - 0x2580) as usize])
    } else {
        None
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
