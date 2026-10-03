//! Color palettes -> 256-entry lookup tables, and the ASCII ramps.
//!
//! The theme table is transcribed 1:1 from the mobile version's
//! `THEMES`/`GEN_THEMES` (ascii_fluid_mobile/index.html) so both ports share
//! the same look. `bands > 1` hard-quantizes the ramp into that many flat
//! colour steps — the limited-palette retro look.

pub type Rgb = (u8, u8, u8);

pub struct Theme {
    pub name: &'static str,
    pub group: &'static str,
    pub bg: u32,
    pub bands: u32,
    pub accent: Rgb,
    pub stops: &'static [(f32, Rgb)],
}

pub const THEMES: &[Theme] = &[
    Theme {
        name: "voyd lime",
        group: "signature",
        bg: pack(10, 12, 9),
        bands: 0,
        accent: (164, 191, 85),
        stops: &[(0.0, (44, 52, 33)), (0.45, (108, 134, 58)), (0.78, (164, 191, 85)), (1.0, (214, 226, 162))],
    },
    Theme {
        name: "phosphor",
        group: "classic",
        bg: pack(0, 3, 0),
        bands: 0,
        accent: (80, 222, 128),
        stops: &[(0.0, (0, 40, 16)), (0.5, (0, 200, 90)), (1.0, (190, 255, 200))],
    },
    Theme {
        name: "amber crt",
        group: "classic",
        bg: pack(10, 6, 0),
        bands: 0,
        accent: (255, 176, 0),
        stops: &[(0.0, (40, 22, 0)), (0.5, (230, 150, 0)), (1.0, (255, 236, 172))],
    },
    Theme {
        name: "ice",
        group: "classic",
        bg: pack(0, 6, 12),
        bands: 0,
        accent: (86, 196, 255),
        stops: &[(0.0, (0, 30, 46)), (0.45, (20, 120, 190)), (0.8, (60, 190, 255)), (1.0, (208, 244, 255))],
    },
    Theme {
        name: "magma",
        group: "classic",
        bg: pack(10, 0, 3),
        bands: 0,
        accent: (255, 120, 40),
        stops: &[(0.0, (20, 0, 8)), (0.35, (150, 20, 10)), (0.62, (255, 90, 0)), (0.82, (255, 200, 0)), (1.0, (255, 255, 235))],
    },
    Theme {
        name: "mono",
        group: "classic",
        bg: pack(0, 0, 0),
        bands: 0,
        accent: (228, 228, 228),
        stops: &[(0.0, (40, 40, 40)), (1.0, (245, 245, 245))],
    },
    Theme {
        name: "Commodore 64 (Pepto)",
        group: "retro computing",
        bg: pack(11, 11, 40),
        bands: 4,
        accent: (108, 94, 181),
        stops: &[(0.0, (53, 40, 121)), (0.4, (111, 61, 134)), (0.75, (108, 94, 181)), (1.0, (255, 255, 255))],
    },
    Theme {
        name: "Commodore PET Phosphor",
        group: "retro computing",
        bg: pack(4, 20, 10),
        bands: 0,
        accent: (72, 246, 80),
        stops: &[(0.0, (0, 36, 12)), (0.4, (0, 132, 36)), (0.75, (40, 210, 72)), (1.0, (72, 246, 80))],
    },
    Theme {
        name: "ZX Spectrum Bright",
        group: "retro computing",
        bg: pack(7, 0, 15),
        bands: 5,
        accent: (255, 0, 255),
        stops: &[(0.0, (0, 0, 215)), (0.28, (255, 0, 255)), (0.55, (255, 0, 0)), (0.8, (255, 255, 0)), (1.0, (255, 255, 255))],
    },
    Theme {
        name: "Apple II Green",
        group: "retro computing",
        bg: pack(0, 20, 0),
        bands: 0,
        accent: (51, 255, 51),
        stops: &[(0.0, (0, 32, 0)), (0.4, (0, 128, 16)), (0.72, (40, 220, 40)), (1.0, (51, 255, 51))],
    },
    Theme {
        name: "Apple II Amber",
        group: "retro computing",
        bg: pack(22, 10, 0),
        bands: 0,
        accent: (255, 176, 0),
        stops: &[(0.0, (36, 16, 0)), (0.4, (140, 66, 0)), (0.72, (224, 140, 0)), (1.0, (255, 176, 0))],
    },
    Theme {
        name: "CGA Cyan / Magenta",
        group: "retro computing",
        bg: pack(2, 9, 11),
        bands: 4,
        accent: (255, 85, 255),
        stops: &[(0.0, (0, 64, 64)), (0.33, (85, 255, 255)), (0.66, (255, 85, 255)), (1.0, (255, 255, 255))],
    },
    Theme {
        name: "CGA Green / Red / Yellow",
        group: "retro computing",
        bg: pack(10, 6, 0),
        bands: 4,
        accent: (255, 255, 85),
        stops: &[(0.0, (20, 50, 0)), (0.33, (85, 255, 85)), (0.66, (255, 85, 85)), (1.0, (255, 255, 85))],
    },
    Theme {
        name: "EGA 16",
        group: "retro computing",
        bg: pack(5, 5, 15),
        bands: 6,
        accent: (85, 255, 255),
        stops: &[(0.0, (0, 0, 170)), (0.2, (170, 0, 170)), (0.4, (255, 85, 85)), (0.6, (255, 255, 85)), (0.8, (85, 255, 85)), (1.0, (255, 255, 255))],
    },
    Theme {
        name: "Amiga Workbench 1.x",
        group: "retro computing",
        bg: pack(0, 0, 34),
        bands: 4,
        accent: (255, 136, 0),
        stops: &[(0.0, (0, 54, 108)), (0.4, (0, 85, 170)), (0.72, (255, 136, 0)), (1.0, (255, 255, 255))],
    },
    Theme {
        name: "Atari 8-bit Blue",
        group: "retro computing",
        bg: pack(4, 16, 42),
        bands: 5,
        accent: (58, 110, 200),
        stops: &[(0.0, (18, 44, 104)), (0.35, (58, 110, 200)), (0.7, (110, 158, 232)), (1.0, (200, 224, 255))],
    },
    Theme {
        name: "MSX1 TMS9918",
        group: "retro computing",
        bg: pack(8, 10, 6),
        bands: 5,
        accent: (252, 85, 84),
        stops: &[(0.0, (33, 176, 59)), (0.25, (33, 200, 66)), (0.5, (212, 193, 84)), (0.75, (252, 85, 84)), (1.0, (255, 255, 255))],
    },
    Theme {
        name: "Hercules Amber",
        group: "retro computing",
        bg: pack(24, 8, 0),
        bands: 0,
        accent: (255, 153, 0),
        stops: &[(0.0, (40, 14, 0)), (0.4, (150, 58, 0)), (0.72, (232, 116, 0)), (1.0, (255, 153, 0))],
    },
    Theme {
        name: "Hercules Green (P1)",
        group: "retro computing",
        bg: pack(1, 26, 8),
        bands: 0,
        accent: (120, 255, 120),
        stops: &[(0.0, (0, 40, 16)), (0.4, (16, 150, 60)), (0.72, (72, 232, 110)), (1.0, (120, 255, 120))],
    },
    Theme {
        name: "DEC VT220 White",
        group: "retro computing",
        bg: pack(10, 11, 14),
        bands: 0,
        accent: (238, 240, 252),
        stops: &[(0.0, (28, 30, 36)), (0.4, (104, 108, 120)), (0.72, (182, 186, 198)), (1.0, (238, 240, 252))],
    },
    Theme {
        name: "Teletext / Ceefax",
        group: "retro computing",
        bg: pack(4, 4, 6),
        bands: 6,
        accent: (0, 255, 255),
        stops: &[(0.0, (0, 0, 255)), (0.2, (255, 0, 0)), (0.4, (255, 0, 255)), (0.6, (0, 255, 255)), (0.8, (255, 255, 0)), (1.0, (255, 255, 255))],
    },
    Theme {
        name: "Game Boy DMG · Pea Soup",
        group: "retro gaming",
        bg: pack(8, 28, 8),
        bands: 4,
        accent: (155, 188, 15),
        stops: &[(0.0, (15, 56, 15)), (0.42, (48, 98, 48)), (0.78, (139, 172, 15)), (1.0, (155, 188, 15))],
    },
    Theme {
        name: "Game Boy Pocket · Ash Grey",
        group: "retro gaming",
        bg: pack(22, 21, 15),
        bands: 4,
        accent: (224, 219, 205),
        stops: &[(0.0, (43, 43, 38)), (0.42, (112, 107, 102)), (0.78, (168, 159, 148)), (1.0, (224, 219, 205))],
    },
    Theme {
        name: "Game Boy Light · Backlit Teal",
        group: "retro gaming",
        bg: pack(7, 26, 20),
        bands: 4,
        accent: (170, 226, 196),
        stops: &[(0.0, (13, 44, 36)), (0.42, (42, 98, 82)), (0.78, (108, 190, 156)), (1.0, (170, 226, 196))],
    },
    Theme {
        name: "Game Boy Color · Prism",
        group: "retro gaming",
        bg: pack(12, 10, 42),
        bands: 0,
        accent: (22, 181, 138),
        stops: &[(0.0, (27, 20, 100)), (0.3, (43, 95, 217)), (0.55, (22, 181, 138)), (0.78, (143, 212, 42)), (1.0, (247, 224, 74))],
    },
    Theme {
        name: "NES · Sky & Cloud",
        group: "retro gaming",
        bg: pack(5, 10, 36),
        bands: 0,
        accent: (60, 188, 252),
        stops: &[(0.0, (0, 35, 124)), (0.32, (0, 0, 252)), (0.62, (60, 188, 252)), (0.85, (164, 228, 252)), (1.0, (252, 252, 252))],
    },
    Theme {
        name: "Master System · Violet Grid",
        group: "retro gaming",
        bg: pack(10, 0, 22),
        bands: 0,
        accent: (170, 0, 255),
        stops: &[(0.0, (0, 0, 170)), (0.32, (85, 0, 170)), (0.6, (170, 0, 255)), (0.82, (255, 85, 255)), (1.0, (255, 170, 255))],
    },
    Theme {
        name: "Virtual Boy · Red Mirage",
        group: "retro gaming",
        bg: pack(20, 0, 0),
        bands: 4,
        accent: (255, 0, 0),
        stops: &[(0.0, (48, 0, 0)), (0.28, (99, 0, 0)), (0.55, (135, 0, 0)), (0.8, (183, 0, 13)), (1.0, (255, 0, 0))],
    },
    Theme {
        name: "PICO-8 · Midnight Cherry",
        group: "retro gaming",
        bg: pack(13, 16, 36),
        bands: 5,
        accent: (255, 0, 77),
        stops: &[(0.0, (29, 43, 83)), (0.35, (126, 37, 83)), (0.62, (255, 0, 77)), (0.84, (255, 119, 168)), (1.0, (255, 204, 170))],
    },
    Theme {
        name: "Neo Geo · Pro-Gear Gold",
        group: "retro gaming",
        bg: pack(20, 6, 10),
        bands: 0,
        accent: (255, 59, 59),
        stops: &[(0.0, (42, 10, 18)), (0.35, (183, 18, 52)), (0.62, (255, 59, 59)), (0.83, (255, 138, 31)), (1.0, (255, 210, 74))],
    },
    Theme {
        name: "WonderSwan · Mono Fog",
        group: "retro gaming",
        bg: pack(14, 16, 13),
        bands: 8,
        accent: (228, 232, 214),
        stops: &[(0.0, (22, 26, 22)), (0.32, (66, 74, 64)), (0.6, (120, 128, 112)), (0.82, (176, 182, 164)), (1.0, (228, 232, 214))],
    },
    Theme {
        name: "Outrun Sunset",
        group: "80s neon",
        bg: pack(21, 8, 38),
        bands: 0,
        accent: (255, 41, 117),
        stops: &[(0.0, (30, 11, 61)), (0.28, (140, 30, 255)), (0.5, (242, 34, 255)), (0.68, (255, 41, 117)), (0.85, (255, 144, 31)), (1.0, (255, 211, 25))],
    },
    Theme {
        name: "Vaporwave Dream",
        group: "80s neon",
        bg: pack(28, 15, 48),
        bands: 0,
        accent: (255, 113, 206),
        stops: &[(0.0, (48, 3, 80)), (0.28, (185, 103, 255)), (0.5, (255, 113, 206)), (0.7, (1, 205, 254)), (0.88, (5, 255, 161)), (1.0, (255, 251, 150))],
    },
    Theme {
        name: "Miami Vice",
        group: "80s neon",
        bg: pack(4, 25, 28),
        bands: 0,
        accent: (255, 110, 199),
        stops: &[(0.0, (4, 28, 32)), (0.3, (0, 128, 128)), (0.55, (11, 210, 211)), (0.8, (255, 110, 199)), (1.0, (249, 144, 232))],
    },
    Theme {
        name: "Tron Grid",
        group: "80s neon",
        bg: pack(1, 2, 8),
        bands: 0,
        accent: (14, 248, 248),
        stops: &[(0.0, (1, 4, 9)), (0.35, (7, 45, 145)), (0.6, (4, 148, 148)), (0.82, (14, 248, 248)), (1.0, (186, 255, 255))],
    },
    Theme {
        name: "Neon Tokyo",
        group: "80s neon",
        bg: pack(6, 7, 19),
        bands: 0,
        accent: (234, 0, 217),
        stops: &[(0.0, (9, 24, 51)), (0.32, (19, 62, 124)), (0.58, (113, 28, 145)), (0.8, (234, 0, 217)), (1.0, (10, 189, 198))],
    },
    Theme {
        name: "Blade Runner Smog",
        group: "80s neon",
        bg: pack(8, 16, 15),
        bands: 0,
        accent: (255, 146, 74),
        stops: &[(0.0, (11, 26, 28)), (0.3, (31, 107, 107)), (0.55, (0, 221, 209)), (0.78, (255, 146, 74)), (1.0, (255, 191, 77))],
    },
    Theme {
        name: "Cyberpunk 2077",
        group: "80s neon",
        bg: pack(10, 8, 16),
        bands: 0,
        accent: (252, 238, 12),
        stops: &[(0.0, (13, 11, 22)), (0.28, (237, 30, 121)), (0.55, (2, 215, 242)), (1.0, (252, 238, 12))],
    },
    Theme {
        name: "Hotline Miami",
        group: "80s neon",
        bg: pack(22, 8, 43),
        bands: 0,
        accent: (247, 101, 184),
        stops: &[(0.0, (42, 10, 61)), (0.3, (123, 45, 142)), (0.55, (247, 101, 184)), (0.8, (39, 253, 245)), (1.0, (168, 246, 248))],
    },
    Theme {
        name: "Terminator Vision",
        group: "80s neon",
        bg: pack(13, 0, 0),
        bands: 0,
        accent: (255, 26, 10),
        stops: &[(0.0, (24, 0, 0)), (0.3, (92, 0, 0)), (0.55, (176, 0, 0)), (0.8, (255, 26, 10)), (1.0, (255, 122, 69))],
    },
    Theme {
        name: "Motion Tracker",
        group: "80s neon",
        bg: pack(0, 21, 5),
        bands: 0,
        accent: (51, 255, 51),
        stops: &[(0.0, (2, 26, 6)), (0.3, (10, 92, 26)), (0.55, (20, 168, 44)), (0.8, (51, 255, 51)), (1.0, (186, 255, 176))],
    },
    Theme {
        name: "CGA Hi-Intensity",
        group: "80s neon",
        bg: pack(5, 3, 8),
        bands: 4,
        accent: (255, 85, 255),
        stops: &[(0.0, (0, 0, 0)), (0.33, (255, 85, 255)), (0.66, (85, 255, 255)), (1.0, (255, 255, 255))],
    },
];

/// Diverging map for the "color by spin" mode (signed vorticity, -1..1).
pub const SPIN_STOPS: &[(f32, Rgb)] = &[
    (-1.0, (60, 200, 255)),
    (-0.4, (20, 80, 120)),
    (-0.05, (16, 20, 14)),
    (0.05, (22, 16, 14)),
    (0.4, (150, 60, 20)),
    (1.0, (255, 140, 40)),
];

pub const fn pack(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

pub const fn pack_rgb(c: Rgb) -> u32 {
    pack(c.0, c.1, c.2)
}

pub const fn unpack(c: u32) -> Rgb {
    (((c >> 16) & 255) as u8, ((c >> 8) & 255) as u8, (c & 255) as u8)
}

fn lerp_stops(stops: &[(f32, Rgb)], t: f32) -> Rgb {
    let first = stops[0];
    let last = stops[stops.len() - 1];
    if t <= first.0 {
        return first.1;
    }
    if t >= last.0 {
        return last.1;
    }
    for pair in stops.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if t >= a.0 && t <= b.0 {
            let f = (t - a.0) / (b.0 - a.0);
            let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * f) as u8;
            return (l(a.1 .0, b.1 .0), l(a.1 .1, b.1 .1), l(a.1 .2, b.1 .2));
        }
    }
    last.1
}

/// Build a 256-entry color table. `signed` maps index 0..255 onto -1..1.
/// `bands > 1` hard-quantizes the (unsigned) ramp into flat colour bands.
pub fn build_lut(stops: &[(f32, Rgb)], signed: bool, bands: u32) -> [u32; 256] {
    let mut lut = [0u32; 256];
    for (k, slot) in lut.iter_mut().enumerate() {
        let mut t = if signed {
            (k as f32 / 255.0) * 2.0 - 1.0
        } else {
            k as f32 / 255.0
        };
        if !signed && bands > 1 {
            let n = (bands - 1) as f32;
            t = (t * n).round() / n;
        }
        let (r, g, b) = lerp_stops(stops, t);
        *slot = pack(r, g, b);
    }
    lut
}

// ---- theme-derived HUD colors (port of the mobile applyUITheme derivation) ----

pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    (l(a.0, b.0), l(a.1, b.1), l(a.2, b.2))
}

/// WCAG relative luminance.
fn rel_lum(c: Rgb) -> f32 {
    let f = |x: u8| {
        let x = x as f32 / 255.0;
        if x <= 0.03928 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
}

/// Lighten toward white until luminance clears a floor — keeps dim accents
/// legible on the dark HUD strip.
pub fn raise_lum(mut c: Rgb, min_l: f32) -> Rgb {
    for _ in 0..40 {
        if rel_lum(c) >= min_l {
            break;
        }
        c = mix(c, (255, 255, 255), 0.08);
    }
    c
}

pub struct Ramp {
    pub name: &'static str,
    pub chars: &'static str,
}

/// Charsets, from the mobile set — minus the ones the embedded 8x8 font can't
/// draw (katakana / half-width kana / kanji / math / runic).
pub const RAMPS: &[Ramp] = &[
    Ramp {
        name: "detailed",
        chars: " .'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$",
    },
    Ramp { name: "standard", chars: " .:-=+*#%@" },
    Ramp { name: "minimal", chars: " .\u{b7}:+#" },
    Ramp { name: "symbols", chars: " .'`,:;!~-_^\"+=<>?/\\|()[]{}*%&$#@" },
    Ramp { name: "digits", chars: " 1742356908" },
    Ramp { name: "hex digits", chars: " 17FC4235EA69D0B8" },
    Ramp { name: "blocks", chars: " \u{2591}\u{2592}\u{2593}\u{2588}" },
    Ramp { name: "dots", chars: " .\u{b7}\u{2022}\u{25cf}\u{2588}" },
    Ramp { name: "braille", chars: " \u{2801}\u{2803}\u{2807}\u{280f}\u{281f}\u{283f}\u{28bf}\u{28ff}" },
    Ramp {
        name: "box & block",
        chars: " \u{2508}\u{2504}\u{254c}\u{2500}\u{2502}\u{253c}\u{2550}\u{256c}\u{2501}\u{2503}\u{254b}\u{2581}\u{2582}\u{2583}\u{2584}\u{2585}\u{2586}\u{2587}\u{2588}",
    },
    Ramp {
        name: "arrows",
        chars: " \u{2191}\u{2192}\u{2193}\u{2190}\u{2194}\u{2195}\u{21d1}\u{21d2}\u{21d3}\u{21d0}\u{27a4}",
    },
    Ramp { name: "stars", chars: " \u{2727}\u{2726}\u{2605}\u{2736}\u{2737}\u{274b}" },
    Ramp {
        name: "hiragana",
        chars: " へしくつてこりいろのんえおかさたなにはひふやゆよらるれをあぬねむめもまほわ",
    },
];
