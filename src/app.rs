//! Application state: parameters, input handling, autopilot, HUD, and the
//! per-frame update that drives the solver and renderer. Feature set mirrors
//! the mobile version (ascii_fluid_mobile) where the platform allows.

use crate::fluid::{Brush, Fluid, BRUSHES};
use crate::palette::{self, RAMPS, THEMES};
use crate::render::{self, Field, View};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window};
use std::path::PathBuf;

const HUD_H: usize = 22;
/// Base body-force scale for gravity (mobile `GRAV`).
const GRAV: f32 = 4.0;

/// Tiny deterministic RNG (xorshift32) — fixed seed keeps the generative
/// scene reproducible across runs, mouse input aside.
struct Rng(u32);
impl Rng {
    fn f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// Adjustable parameters, cycled with Tab and nudged with the arrow keys.
#[derive(Clone, Copy)]
enum Param {
    Speed,
    Vorticity,
    Viscosity,
    DyeSpread,
    Persistence,
    BrushForce,
    Gravity,
    Contrast,
    Detail,
    Seeds,
}
const PARAMS: [Param; 10] = [
    Param::Speed, Param::Vorticity, Param::Viscosity, Param::DyeSpread, Param::Persistence,
    Param::BrushForce, Param::Gravity, Param::Contrast, Param::Detail, Param::Seeds,
];

/// Spin direction for new vortices: alternate each click, or lock a direction.
#[derive(Clone, Copy, PartialEq)]
enum SpinMode {
    Auto,
    Cw,
    Ccw,
}
impl SpinMode {
    fn name(self) -> &'static str {
        match self {
            SpinMode::Auto => "auto",
            SpinMode::Cw => "cw",
            SpinMode::Ccw => "ccw",
        }
    }
    fn next(self) -> Self {
        match self {
            SpinMode::Auto => SpinMode::Cw,
            SpinMode::Cw => SpinMode::Ccw,
            SpinMode::Ccw => SpinMode::Auto,
        }
    }
    fn from_name(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(SpinMode::Auto),
            "cw" => Some(SpinMode::Cw),
            "ccw" => Some(SpinMode::Ccw),
            _ => None,
        }
    }
}

pub struct App {
    pub quit: bool,
    paused: bool,
    hud: bool,

    fluid: Fluid,
    rng: Rng,

    // window / grid geometry
    width: usize,
    height: usize,
    cell: usize,
    cols: usize,
    rows: usize,
    off_x: usize,
    off_y: usize,

    // slider-style parameters (same ranges as the web version)
    speed: i32,
    vort: i32,
    visc: i32,
    diff: i32,
    fade_pct: i32,
    force: i32,
    grav: i32,
    contrast: i32,
    detail: i32,
    count: i32,

    // discrete options
    field: Field,
    theme_idx: usize,
    ramp_idx: usize,
    brush_idx: usize,
    spin_mode: SpinMode,
    invert: bool,
    spin_color: bool,
    autopilot: bool,
    gravity: bool,

    // derived render state
    lut: [u32; 256],
    bg: u32,
    ramp_chars: Vec<char>,
    run_max: f32,

    // interaction state
    selected: usize,
    adj_cool: i32,
    mouse_down: bool,
    last_cell: (usize, usize),
    click_parity: f32,
    auto_acc: f32,
}

impl App {
    pub fn new(width: usize, height: usize) -> Self {
        let mut app = App {
            quit: false,
            paused: false,
            hud: true,
            fluid: Fluid::new(8, 8),
            rng: Rng(0x1234_5678),
            width,
            height,
            cell: 9,
            cols: 0,
            rows: 0,
            off_x: 0,
            off_y: 0,
            speed: 33,
            vort: 60,
            visc: 12,
            diff: 8,
            fade_pct: 99,
            force: 50,
            grav: 50,
            contrast: 72,
            detail: 9,
            count: 5,
            field: Field::Dye,
            theme_idx: 0,
            ramp_idx: 0,
            brush_idx: 0,
            spin_mode: SpinMode::Auto,
            invert: false,
            spin_color: false,
            autopilot: true,
            gravity: false,
            lut: [0; 256],
            bg: THEMES[0].bg,
            ramp_chars: Vec::new(),
            run_max: 1e-3,
            selected: 1, // start on vorticity — the fun knob
            adj_cool: 0,
            mouse_down: false,
            last_cell: (1, 1),
            click_parity: 1.0,
            auto_acc: 0.0,
        };
        app.load_config();
        app.ramp_chars = RAMPS[app.ramp_idx].chars.chars().collect();
        app.rebuild_lut();
        app.recompute_grid(); // also seeds
        app
    }

    // ---- derived parameter mappings ----
    fn dt(&self) -> f32 {
        0.02 + (self.speed as f32 / 100.0) * 0.28
    }
    fn force_scale(&self) -> f32 {
        (self.force as f32 / 100.0) * 60.0
    }
    fn gamma(&self) -> f32 {
        self.contrast as f32 / 100.0
    }
    fn apply_params(&mut self) {
        self.fluid.eps = (self.vort as f32 / 100.0) * 20.0;
        self.fluid.visc = (self.visc as f32 / 100.0).powi(2) * 2e-3;
        self.fluid.diff = (self.diff as f32 / 100.0).powi(2) * 2e-3;
        self.fluid.fade = self.fade_pct as f32 / 100.0;
    }

    fn rebuild_lut(&mut self) {
        let theme = &THEMES[self.theme_idx];
        self.bg = theme.bg;
        self.lut = if self.spin_color && self.field == Field::Vort {
            palette::build_lut(palette::SPIN_STOPS, true, 0)
        } else {
            palette::build_lut(theme.stops, false, theme.bands)
        };
    }

    fn recompute_grid(&mut self) {
        self.cell = self.detail as usize;
        let usable_h = self.height.saturating_sub(HUD_H);
        let cols = (self.width / self.cell).max(8);
        let rows = (usable_h / self.cell).max(8);
        self.off_x = (self.width - cols * self.cell) / 2;
        self.off_y = (usable_h - rows * self.cell) / 2;
        if cols != self.cols || rows != self.rows {
            self.cols = cols;
            self.rows = rows;
            self.fluid.resize(cols, rows);
            self.reseed();
        }
    }

    pub fn set_window_size(&mut self, width: usize, height: usize) {
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.recompute_grid();
        }
    }

    /// Inject the selected brush at (cx, cy) — used by clicks, seeding, autopilot.
    fn splat(&mut self, cx: f32, cy: f32, spin: f32, scale: f32) {
        let brush = BRUSHES[self.brush_idx];
        let rng = &mut self.rng;
        self.fluid.apply_brush(brush, cx, cy, spin, scale, &mut || rng.f32());
    }

    fn reseed(&mut self) {
        self.fluid.clear_all();
        self.run_max = 1e-3;
        let n = self.count.max(1);
        for k in 0..n {
            let a = (k as f32 / n as f32) * std::f32::consts::TAU;
            let jit = self.rng.f32() * 2.0 - 1.0;
            let cx = self.cols as f32 * (0.5 + 0.36 * a.cos()) + jit * self.cols as f32 * 0.05;
            let cy = self.rows as f32 * (0.5 + 0.36 * a.sin()) + jit * self.rows as f32 * 0.05;
            let spin = if k % 2 == 0 { 1.0 } else { -1.0 };
            self.splat(cx, cy, spin, 1.0);
        }
    }

    /// Random theme + charset + brush, then reseed (mobile "randomize ⚄").
    fn randomize(&mut self) {
        self.theme_idx = (self.rng.f32() * THEMES.len() as f32) as usize % THEMES.len();
        self.ramp_idx = (self.rng.f32() * RAMPS.len() as f32) as usize % RAMPS.len();
        self.brush_idx = (self.rng.f32() * BRUSHES.len() as f32) as usize % BRUSHES.len();
        self.ramp_chars = RAMPS[self.ramp_idx].chars.chars().collect();
        self.rebuild_lut();
        self.reseed();
    }

    /// Spin for the next click: alternate in auto mode, else the locked direction.
    fn next_spin(&mut self) -> f32 {
        match self.spin_mode {
            SpinMode::Cw => 1.0,
            SpinMode::Ccw => -1.0,
            SpinMode::Auto => {
                let s = self.click_parity;
                self.click_parity = -self.click_parity;
                s
            }
        }
    }

    // ------------------------------------------------------------------
    // input
    // ------------------------------------------------------------------
    pub fn handle_input(&mut self, win: &Window) {
        let shift = win.is_key_down(Key::LeftShift) || win.is_key_down(Key::RightShift);

        for key in win.get_keys_pressed(KeyRepeat::No) {
            match key {
                Key::Escape | Key::Q => self.quit = true,
                Key::Space => self.paused = !self.paused,
                Key::R => {
                    if shift {
                        self.randomize();
                    } else {
                        self.reseed();
                    }
                }
                Key::C => {
                    self.fluid.clear_dye();
                    self.run_max = 1e-3;
                }
                Key::F => {
                    self.field = match self.field {
                        Field::Dye => Field::Speed,
                        Field::Speed => Field::Vort,
                        Field::Vort => Field::Dye,
                    };
                    self.rebuild_lut();
                }
                Key::T => {
                    let n = THEMES.len();
                    self.theme_idx = if shift { (self.theme_idx + n - 1) % n } else { (self.theme_idx + 1) % n };
                    self.rebuild_lut();
                }
                Key::G => {
                    let n = RAMPS.len();
                    self.ramp_idx = if shift { (self.ramp_idx + n - 1) % n } else { (self.ramp_idx + 1) % n };
                    self.ramp_chars = RAMPS[self.ramp_idx].chars.chars().collect();
                }
                Key::B => {
                    let n = BRUSHES.len();
                    self.brush_idx = if shift { (self.brush_idx + n - 1) % n } else { (self.brush_idx + 1) % n };
                }
                Key::D => self.spin_mode = self.spin_mode.next(),
                Key::W => self.gravity = !self.gravity,
                Key::I => self.invert = !self.invert,
                Key::S => {
                    self.spin_color = !self.spin_color;
                    self.rebuild_lut();
                }
                Key::A => self.autopilot = !self.autopilot,
                Key::H => self.hud = !self.hud,
                Key::Tab => {
                    let n = PARAMS.len();
                    self.selected = if shift {
                        (self.selected + n - 1) % n
                    } else {
                        (self.selected + 1) % n
                    };
                }
                _ => {}
            }
        }

        // held arrow / bracket adjustment with a small repeat cooldown
        if self.adj_cool > 0 {
            self.adj_cool -= 1;
        }
        let dec = win.is_key_down(Key::Left) || win.is_key_down(Key::LeftBracket) || win.is_key_down(Key::Minus);
        let inc = win.is_key_down(Key::Right) || win.is_key_down(Key::RightBracket) || win.is_key_down(Key::Equal);
        if self.adj_cool == 0 && (dec || inc) {
            self.adjust(if inc { 1 } else { -1 });
            self.adj_cool = 3;
        }

        self.handle_mouse(win, shift);
    }

    fn adjust(&mut self, dir: i32) {
        let p = PARAMS[self.selected];
        // (value, min, max, step)
        let (val, min, max, step): (&mut i32, i32, i32, i32) = match p {
            Param::Speed => (&mut self.speed, 5, 100, 5),
            Param::Vorticity => (&mut self.vort, 0, 100, 5),
            Param::Viscosity => (&mut self.visc, 0, 100, 5),
            Param::DyeSpread => (&mut self.diff, 0, 100, 5),
            Param::Persistence => (&mut self.fade_pct, 90, 100, 1),
            Param::BrushForce => (&mut self.force, 0, 100, 5),
            Param::Gravity => (&mut self.grav, 0, 100, 5),
            Param::Contrast => (&mut self.contrast, 30, 140, 5),
            Param::Detail => (&mut self.detail, 6, 20, 1),
            Param::Seeds => (&mut self.count, 1, 12, 1),
        };
        *val = (*val + dir * step).clamp(min, max);
        if let Param::Detail = p {
            self.recompute_grid();
        }
    }

    fn handle_mouse(&mut self, win: &Window, shift: bool) {
        let down = win.get_mouse_down(MouseButton::Left);
        let pos = win.get_mouse_pos(MouseMode::Discard);

        if let Some((mx, my)) = pos {
            let cell = self.cell as f32;
            let i = (((mx - self.off_x as f32) / cell) as i32 + 1).clamp(1, self.cols as i32) as usize;
            let j = (((my - self.off_y as f32) / cell) as i32 + 1).clamp(1, self.rows as i32) as usize;

            if down && !self.mouse_down {
                // press edge -> inject the selected brush
                let mut spin = self.next_spin();
                if shift {
                    spin = -spin;
                }
                self.splat(i as f32, j as f32, spin, 1.0);
                self.last_cell = (i, j);
            } else if down {
                // drag -> push the fluid
                let di = i as f32 - self.last_cell.0 as f32;
                let dj = j as f32 - self.last_cell.1 as f32;
                let fs = self.force_scale();
                self.fluid.add_force(i, j, di * fs, dj * fs, 18.0, 2);
                self.last_cell = (i, j);
            }
        }
        self.mouse_down = down;
    }

    // ------------------------------------------------------------------
    // update + render
    // ------------------------------------------------------------------
    pub fn step(&mut self) {
        if self.paused {
            return;
        }
        self.apply_params();
        if self.gravity {
            self.fluid.add_uniform(0.0, (self.grav as f32 / 100.0) * GRAV);
        }
        self.fluid.step(self.dt());
    }

    pub fn autopilot(&mut self, dt_real: f32) {
        if !self.autopilot || self.paused {
            return;
        }
        self.auto_acc += dt_real;
        if self.auto_acc >= 2.2 {
            self.auto_acc = 0.0;
            let cx = 2.0 + self.rng.f32() * (self.cols as f32 - 4.0);
            let cy = 2.0 + self.rng.f32() * (self.rows as f32 - 4.0);
            let spin = if self.rng.f32() < 0.5 { -1.0 } else { 1.0 };
            self.splat(cx, cy, spin, 0.7);
        }
    }

    pub fn render(&mut self, buf: &mut [u32], width: usize, height: usize, fps: f32) {
        let m = render::scan_max(&self.fluid, self.field, self.cols, self.rows);
        self.run_max = m.max(self.run_max * 0.94).max(1e-4);

        render::fill(buf, self.bg);
        let view = View {
            lut: &self.lut,
            ramp: &self.ramp_chars,
            field: self.field,
            invert: self.invert,
            gamma: self.gamma(),
            spin: self.spin_color,
            cell: self.cell,
            cols: self.cols,
            rows: self.rows,
            off_x: self.off_x,
            off_y: self.off_y,
            inv_norm: 1.0 / self.run_max,
        };
        render::draw_field(buf, width, height, &self.fluid, &view);

        if self.hud {
            self.draw_hud(buf, width, height, fps);
        }
    }

    fn draw_hud(&self, buf: &mut [u32], width: usize, height: usize, fps: f32) {
        // HUD colors follow the active palette (port of the mobile UI theming).
        let theme = &THEMES[self.theme_idx];
        let bg = palette::unpack(theme.bg);
        let accent = palette::pack_rgb(theme.accent);
        let dim = palette::pack_rgb(palette::raise_lum(palette::mix(theme.accent, bg, 0.40), 0.22));
        let strip = palette::pack_rgb(palette::mix(bg, (0, 0, 0), 0.3));
        let y = height.saturating_sub(HUD_H);
        render::fill_rect(buf, width, height, 0, y, width, HUD_H, strip);

        let field = match self.field {
            Field::Dye => "dye",
            Field::Speed => "speed",
            Field::Vort => "vorticity",
        };
        let sel = format!("[{}: {}]", self.sel_label(), self.sel_value());
        let status = format!(
            "field {}  {} \u{b7} {}  brush {}  {}seed  {}fps{}{}{}{}",
            field,
            theme.group,
            theme.name,
            BRUSHES[self.brush_idx].name(),
            self.count,
            fps.round() as i32,
            if self.paused { "  PAUSED" } else { "" },
            if self.autopilot { "" } else { "  auto-off" },
            if self.gravity { "  grav" } else { "" },
            match self.spin_mode {
                SpinMode::Auto => "",
                SpinMode::Cw => "  spin:cw",
                SpinMode::Ccw => "  spin:ccw",
            },
        );
        let ty = y + (HUD_H - 8) / 2;
        render::blit_text(buf, width, height, 8, ty, 8, &sel, accent);
        let x2 = 8 + (sel.chars().count() + 2) * 8;
        render::blit_text(buf, width, height, x2, ty, 8, &status, dim);

        let hint = "tab:param </>:adj  f:field t:pal g:chars b:brush d:spin w:grav i:inv s:spinclr a:auto  shift+r:rand r:reseed c:clear h:hide";
        let hx = width.saturating_sub(hint.chars().count() * 6 + 8);
        if hx > x2 + status.chars().count() * 8 + 16 {
            render::blit_text(buf, width, height, hx, ty, 6, hint, dim);
        }
    }

    fn sel_label(&self) -> &'static str {
        match PARAMS[self.selected] {
            Param::Speed => "speed",
            Param::Vorticity => "vorticity",
            Param::Viscosity => "viscosity",
            Param::DyeSpread => "dye spread",
            Param::Persistence => "persistence",
            Param::BrushForce => "brush force",
            Param::Gravity => "gravity",
            Param::Contrast => "contrast",
            Param::Detail => "detail",
            Param::Seeds => "seeds",
        }
    }
    fn sel_value(&self) -> i32 {
        match PARAMS[self.selected] {
            Param::Speed => self.speed,
            Param::Vorticity => self.vort,
            Param::Viscosity => self.visc,
            Param::DyeSpread => self.diff,
            Param::Persistence => self.fade_pct,
            Param::BrushForce => self.force,
            Param::Gravity => self.grav,
            Param::Contrast => self.contrast,
            Param::Detail => self.detail,
            Param::Seeds => self.count,
        }
    }

    // ------------------------------------------------------------------
    // settings persistence (the desktop analog of the mobile localStorage)
    // ------------------------------------------------------------------
    fn config_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))?;
        Some(base.join("ascii_fluid").join("config"))
    }

    fn load_config(&mut self) {
        let Some(path) = Self::config_path() else { return };
        let Ok(text) = std::fs::read_to_string(path) else { return };
        for line in text.lines() {
            let Some((key, val)) = line.split_once('=') else { continue };
            let (key, val) = (key.trim(), val.trim());
            let int = |lo: i32, hi: i32| val.parse::<i32>().ok().map(|v| v.clamp(lo, hi));
            match key {
                "speed" => self.speed = int(5, 100).unwrap_or(self.speed),
                "vorticity" => self.vort = int(0, 100).unwrap_or(self.vort),
                "viscosity" => self.visc = int(0, 100).unwrap_or(self.visc),
                "dye_spread" => self.diff = int(0, 100).unwrap_or(self.diff),
                "persistence" => self.fade_pct = int(90, 100).unwrap_or(self.fade_pct),
                "brush_force" => self.force = int(0, 100).unwrap_or(self.force),
                "gravity_pull" => self.grav = int(0, 100).unwrap_or(self.grav),
                "contrast" => self.contrast = int(30, 140).unwrap_or(self.contrast),
                "detail" => self.detail = int(6, 20).unwrap_or(self.detail),
                "seeds" => self.count = int(1, 12).unwrap_or(self.count),
                "field" => {
                    self.field = match val {
                        "speed" => Field::Speed,
                        "vorticity" => Field::Vort,
                        _ => Field::Dye,
                    }
                }
                // stored by name; silently keep the default if the entry is stale
                "palette" => {
                    if let Some(i) = THEMES.iter().position(|t| t.name == val) {
                        self.theme_idx = i;
                    }
                }
                "charset" => {
                    if let Some(i) = RAMPS.iter().position(|r| r.name == val) {
                        self.ramp_idx = i;
                    }
                }
                "brush" => {
                    if let Some(b) = Brush::from_name(val) {
                        self.brush_idx = BRUSHES.iter().position(|&x| x == b).unwrap_or(self.brush_idx);
                    }
                }
                "spin_mode" => self.spin_mode = SpinMode::from_name(val).unwrap_or(self.spin_mode),
                "invert" => self.invert = val == "1",
                "spin_colors" => self.spin_color = val == "1",
                "autopilot" => self.autopilot = val == "1",
                "hud" => self.hud = val == "1",
                _ => {}
            }
        }
    }

    pub fn save_config(&self) {
        let Some(path) = Self::config_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let field = match self.field {
            Field::Dye => "dye",
            Field::Speed => "speed",
            Field::Vort => "vorticity",
        };
        let b = |x: bool| if x { "1" } else { "0" };
        let text = format!(
            "speed={}\nvorticity={}\nviscosity={}\ndye_spread={}\npersistence={}\n\
             brush_force={}\ngravity_pull={}\ncontrast={}\ndetail={}\nseeds={}\n\
             field={}\npalette={}\ncharset={}\nbrush={}\nspin_mode={}\n\
             invert={}\nspin_colors={}\nautopilot={}\nhud={}\n",
            self.speed, self.vort, self.visc, self.diff, self.fade_pct,
            self.force, self.grav, self.contrast, self.detail, self.count,
            field, THEMES[self.theme_idx].name, RAMPS[self.ramp_idx].name,
            BRUSHES[self.brush_idx].key(), self.spin_mode.name(),
            b(self.invert), b(self.spin_color), b(self.autopilot), b(self.hud),
        );
        let _ = std::fs::write(path, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trip() {
        // isolate from the user's real config
        let dir = std::env::temp_dir().join(format!("ascii_fluid_test_{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &dir);

        let mut a = App::new(800, 600);
        a.speed = 75;
        a.fade_pct = 95;
        a.theme_idx = 7;
        a.ramp_idx = 8;
        a.brush_idx = 5;
        a.spin_mode = SpinMode::Ccw;
        a.field = Field::Vort;
        a.invert = true;
        a.autopilot = false;
        a.paused = true; // must NOT persist
        a.gravity = true; // must NOT persist
        a.save_config();

        let b = App::new(800, 600);
        assert_eq!(b.speed, 75);
        assert_eq!(b.fade_pct, 95);
        assert_eq!(b.theme_idx, 7);
        assert_eq!(b.ramp_idx, 8);
        assert_eq!(b.brush_idx, 5);
        assert!(b.spin_mode == SpinMode::Ccw);
        assert!(b.field == Field::Vort);
        assert!(b.invert);
        assert!(!b.autopilot);
        assert!(!b.paused);
        assert!(!b.gravity);

        // out-of-range and stale values fall back safely
        let path = App::config_path().unwrap();
        std::fs::write(&path, "speed=9999\npalette=no such theme\nbrush=nonsense\ndetail=1\n").unwrap();
        let c = App::new(800, 600);
        assert_eq!(c.speed, 100); // clamped
        assert_eq!(c.theme_idx, 0); // default kept
        assert_eq!(c.brush_idx, 0);
        assert_eq!(c.detail, 6); // clamped

        let _ = std::fs::remove_dir_all(dir);
    }
}
