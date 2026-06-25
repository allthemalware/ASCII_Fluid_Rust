//! Color palettes -> 256-entry lookup tables, and the ASCII ramps.

pub type Rgb = (u8, u8, u8);

pub struct Theme {
    pub name: &'static str,
    pub bg: u32,
    pub stops: &'static [(f32, Rgb)],
}

pub const THEMES: &[Theme] = &[
    Theme {
        name: "voyd lime",
        bg: pack(10, 12, 9),
        stops: &[
            (0.0, (44, 52, 33)),
            (0.45, (108, 134, 58)),
            (0.78, (164, 191, 85)),
            (1.0, (214, 226, 162)),
        ],
    },
    Theme {
        name: "phosphor",
        bg: pack(0, 3, 0),
        stops: &[(0.0, (0, 40, 16)), (0.5, (0, 200, 90)), (1.0, (190, 255, 200))],
    },
    Theme {
        name: "amber crt",
        bg: pack(10, 6, 0),
        stops: &[(0.0, (40, 22, 0)), (0.5, (230, 150, 0)), (1.0, (255, 236, 172))],
    },
    Theme {
        name: "ice",
        bg: pack(0, 6, 12),
        stops: &[
            (0.0, (0, 30, 46)),
            (0.45, (20, 120, 190)),
            (0.8, (60, 190, 255)),
            (1.0, (208, 244, 255)),
        ],
    },
    Theme {
        name: "magma",
        bg: pack(10, 0, 3),
        stops: &[
            (0.0, (20, 0, 8)),
            (0.35, (150, 20, 10)),
            (0.62, (255, 90, 0)),
            (0.82, (255, 200, 0)),
            (1.0, (255, 255, 235)),
        ],
    },
    Theme {
        name: "mono",
        bg: pack(0, 0, 0),
        stops: &[(0.0, (40, 40, 40)), (1.0, (245, 245, 245))],
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
pub fn build_lut(stops: &[(f32, Rgb)], signed: bool) -> [u32; 256] {
    let mut lut = [0u32; 256];
    for (k, slot) in lut.iter_mut().enumerate() {
        let t = if signed {
            (k as f32 / 255.0) * 2.0 - 1.0
        } else {
            k as f32 / 255.0
        };
        let (r, g, b) = lerp_stops(stops, t);
        *slot = pack(r, g, b);
    }
    lut
}

pub struct Ramp {
    pub name: &'static str,
    pub chars: &'static str,
}

pub const RAMPS: &[Ramp] = &[
    Ramp {
        name: "detailed",
        chars: " .'`^\",:;Il!i><~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$",
    },
    Ramp { name: "standard", chars: " .:-=+*#%@" },
    Ramp { name: "minimal", chars: " .:+#" },
    Ramp { name: "blocks", chars: " \u{2591}\u{2592}\u{2593}\u{2588}" },
];
