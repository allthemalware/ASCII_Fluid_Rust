//! Application state: parameters, input handling, autopilot, and the
//! per-frame update that drives the solver and renderer.

use crate::fluid::Fluid;
use crate::palette::{self, RAMPS, THEMES};
use crate::render::{self, Field, View};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window};

const HUD_H: usize = 22;

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
    Contrast,
    Detail,
    Vortices,
}
const PARAMS: [Param; 9] = [
    Param::Speed, Param::Vorticity, Param::Viscosity, Param::DyeSpread,
    Param::Persistence, Param::BrushForce, Param::Contrast, Param::Detail, Param::Vortices,
];

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
    contrast: i32,
    detail: i32,
    count: i32,

    // discrete options
    field: Field,
    theme_idx: usize,
    ramp_idx: usize,
    invert: bool,
    spin: bool,
    autopilot: bool,

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
            contrast: 72,
            detail: 9,
            count: 5,
            field: Field::Dye,
            theme_idx: 0,
            ramp_idx: 0,
            invert: false,
            spin: false,
            autopilot: true,
            lut: [0; 256],
            bg: THEMES[0].bg,
            ramp_chars: RAMPS[0].chars.chars().collect(),
            run_max: 1e-3,
            selected: 1, // start on vorticity — the fun knob
            adj_cool: 0,
            mouse_down: false,
            last_cell: (1, 1),
            click_parity: 1.0,
            auto_acc: 0.0,
        };
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
        self.lut = if self.spin && self.field == Field::Vort {
            palette::build_lut(palette::SPIN_STOPS, true)
        } else {
            palette::build_lut(theme.stops, false)
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

    fn reseed(&mut self) {
        self.fluid.clear_all();
        self.run_max = 1e-3;
        let n = self.count.max(1);
        let rad = (self.cols.min(self.rows) as f32 * 0.09).clamp(3.0, 14.0);
        for k in 0..n {
            let a = (k as f32 / n as f32) * std::f32::consts::TAU;
            let jit = self.rng.f32() * 2.0 - 1.0;
            let cx = self.cols as f32 * (0.5 + 0.36 * a.cos()) + jit * self.cols as f32 * 0.05;
            let cy = self.rows as f32 * (0.5 + 0.36 * a.sin()) + jit * self.rows as f32 * 0.05;
            let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
            self.fluid.splat_vortex(cx, cy, sign * 0.95, rad, 1.0);
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
                Key::R => self.reseed(),
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
                    self.theme_idx = (self.theme_idx + 1) % THEMES.len();
                    self.rebuild_lut();
                }
                Key::G => {
                    self.ramp_idx = (self.ramp_idx + 1) % RAMPS.len();
                    self.ramp_chars = RAMPS[self.ramp_idx].chars.chars().collect();
                }
                Key::I => self.invert = !self.invert,
                Key::S => {
                    self.spin = !self.spin;
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
            Param::Contrast => (&mut self.contrast, 30, 140, 5),
            Param::Detail => (&mut self.detail, 6, 20, 1),
            Param::Vortices => (&mut self.count, 1, 12, 1),
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
                // press edge -> spawn a vortex
                let sign = if shift { -self.click_parity } else { self.click_parity };
                self.click_parity = -self.click_parity;
                let rad = (self.cols.min(self.rows) as f32 * 0.07).clamp(3.0, 11.0);
                self.fluid.splat_vortex(i as f32, j as f32, sign * 1.15, rad, 1.2);
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
            let rad = (self.cols.min(self.rows) as f32 * 0.06).clamp(3.0, 9.0);
            let sign = if self.rng.f32() < 0.5 { -1.0 } else { 1.0 };
            self.fluid.splat_vortex(cx, cy, sign * 0.5, rad, 0.7);
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
            spin: self.spin,
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
        let lime = palette::pack(164, 191, 85);
        let dim = palette::pack(108, 134, 58);
        let strip = palette::pack(6, 8, 5);
        let y = height.saturating_sub(HUD_H);
        render::fill_rect(buf, width, height, 0, y, width, HUD_H, strip);

        let field = match self.field {
            Field::Dye => "dye",
            Field::Speed => "speed",
            Field::Vort => "vorticity",
        };
        let sel = format!("[{}: {}]", self.sel_label(), self.sel_value());
        let status = format!(
            "field {}  palette {}  {}vtx  {}fps{}{}",
            field,
            THEMES[self.theme_idx].name,
            self.count,
            fps.round() as i32,
            if self.paused { "  PAUSED" } else { "" },
            if self.autopilot { "" } else { "  auto-off" },
        );
        let ty = y + (HUD_H - 8) / 2;
        render::blit_text(buf, width, height, 8, ty, 8, &sel, lime);
        let x2 = 8 + (sel.chars().count() + 2) * 8;
        render::blit_text(buf, width, height, x2, ty, 8, &status, dim);

        let hint = "tab:param  </>:adjust  f:field t:palette g:chars i:invert s:spin a:auto  click:vortex drag:push  r:reseed c:clear h:hide";
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
            Param::Contrast => "contrast",
            Param::Detail => "detail",
            Param::Vortices => "vortices",
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
            Param::Contrast => self.contrast,
            Param::Detail => self.detail,
            Param::Vortices => self.count,
        }
    }
}
