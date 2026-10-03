//! Stable-fluids solver (Jos Stam, "Real-Time Fluid Dynamics for Games")
//! plus Fedkiw vorticity confinement, which is what keeps the vortices crisp
//! and alive in slow motion instead of numerically diffusing away.
//!
//! Grids are stored as flat `Vec<f32>` of size `(nx+2)*(ny+2)` — a one-cell
//! border around the `nx*ny` interior carries the boundary conditions.

/// The distortion type injected by clicks / seeding / autopilot — same set as
/// the mobile version's brush picker.
#[derive(Clone, Copy, PartialEq)]
pub enum Brush {
    Vortex,
    Burst,
    Implode,
    Jet,
    Dipole,
    Shock,
    Turbulence,
    Bloom,
    Shear,
}

pub const BRUSHES: [Brush; 9] = [
    Brush::Vortex,
    Brush::Burst,
    Brush::Implode,
    Brush::Jet,
    Brush::Dipole,
    Brush::Shock,
    Brush::Turbulence,
    Brush::Bloom,
    Brush::Shear,
];

impl Brush {
    /// Display name (HUD).
    pub fn name(self) -> &'static str {
        match self {
            Brush::Vortex => "vortex",
            Brush::Burst => "burst",
            Brush::Implode => "implode",
            Brush::Jet => "jet",
            Brush::Dipole => "dipole",
            Brush::Shock => "shockwave",
            Brush::Turbulence => "turbulence",
            Brush::Bloom => "ink bloom",
            Brush::Shear => "shear",
        }
    }

    /// Stable short key (config file, Python API) — the mobile object keys.
    pub fn key(self) -> &'static str {
        match self {
            Brush::Vortex => "vortex",
            Brush::Burst => "burst",
            Brush::Implode => "implode",
            Brush::Jet => "jet",
            Brush::Dipole => "dipole",
            Brush::Shock => "shock",
            Brush::Turbulence => "turbulence",
            Brush::Bloom => "bloom",
            Brush::Shear => "shear",
        }
    }

    pub fn from_name(s: &str) -> Option<Brush> {
        BRUSHES.iter().copied().find(|b| b.key() == s || b.name() == s)
    }
}

#[derive(Clone, Copy)]
struct Dims {
    nx: usize,
    ny: usize,
    w: usize, // row stride = nx + 2
}

#[inline(always)]
fn ix(d: Dims, i: usize, j: usize) -> usize {
    i + d.w * j
}

pub struct Fluid {
    pub nx: usize,
    pub ny: usize,
    w: usize,
    size: usize,

    pub iters: usize,
    pub visc: f32,
    pub diff: f32,
    pub eps: f32,  // vorticity confinement strength
    pub fade: f32, // dye dissipation per step

    pub u: Vec<f32>,
    pub v: Vec<f32>,
    u0: Vec<f32>,
    v0: Vec<f32>,
    pub d: Vec<f32>,
    d0: Vec<f32>,
    u_src: Vec<f32>,
    v_src: Vec<f32>,
    d_src: Vec<f32>,
    pub curl: Vec<f32>,
}

impl Fluid {
    pub fn new(nx: usize, ny: usize) -> Self {
        let mut f = Fluid {
            nx: 0,
            ny: 0,
            w: 0,
            size: 0,
            iters: 16,
            visc: 2e-5,
            diff: 1e-5,
            eps: 12.0,
            fade: 0.992,
            u: Vec::new(),
            v: Vec::new(),
            u0: Vec::new(),
            v0: Vec::new(),
            d: Vec::new(),
            d0: Vec::new(),
            u_src: Vec::new(),
            v_src: Vec::new(),
            d_src: Vec::new(),
            curl: Vec::new(),
        };
        f.resize(nx, ny);
        f
    }

    pub fn resize(&mut self, nx: usize, ny: usize) {
        self.nx = nx;
        self.ny = ny;
        self.w = nx + 2;
        self.size = (nx + 2) * (ny + 2);
        let z = || vec![0.0f32; self.size];
        self.u = z();
        self.v = z();
        self.u0 = z();
        self.v0 = z();
        self.d = z();
        self.d0 = z();
        self.u_src = z();
        self.v_src = z();
        self.d_src = z();
        self.curl = z();
    }

    fn dims(&self) -> Dims {
        Dims { nx: self.nx, ny: self.ny, w: self.w }
    }

    #[inline]
    fn ix(&self, i: usize, j: usize) -> usize {
        i + self.w * j
    }

    pub fn clear_all(&mut self) {
        for buf in [
            &mut self.u, &mut self.v, &mut self.d,
            &mut self.u_src, &mut self.v_src, &mut self.d_src,
        ] {
            buf.iter_mut().for_each(|x| *x = 0.0);
        }
    }

    pub fn clear_dye(&mut self) {
        self.d.iter_mut().for_each(|x| *x = 0.0);
    }

    fn clear_sources(&mut self) {
        self.u_src.iter_mut().for_each(|x| *x = 0.0);
        self.v_src.iter_mut().for_each(|x| *x = 0.0);
        self.d_src.iter_mut().for_each(|x| *x = 0.0);
    }

    // ---- public field accessors used by the renderer ----
    // (speed/vorticity are only called from the window binary, so the lib
    // target sees them as dead code)
    #[inline]
    pub fn dye(&self, i: usize, j: usize) -> f32 {
        self.d[self.ix(i, j)]
    }
    #[allow(dead_code)]
    #[inline]
    pub fn speed(&self, i: usize, j: usize) -> f32 {
        let k = self.ix(i, j);
        self.u[k].abs() + self.v[k].abs()
    }
    #[allow(dead_code)]
    #[inline]
    pub fn vorticity(&self, i: usize, j: usize) -> f32 {
        self.curl[self.ix(i, j)]
    }

    // ------------------------------------------------------------------
    // simulation step
    // ------------------------------------------------------------------
    pub fn step(&mut self, dt: f32) {
        self.vel_step(dt);
        self.dens_step(dt);
        if self.eps <= 0.0 {
            // confinement normally fills curl[]; keep it current for the view.
            compute_curl(self.dims(), &self.u, &self.v, &mut self.curl);
        }
        self.clear_sources();
    }

    fn vel_step(&mut self, dt: f32) {
        let d = self.dims();
        add_source(&mut self.u, &self.u_src, dt);
        add_source(&mut self.v, &self.v_src, dt);
        vort_confine(d, &mut self.u, &mut self.v, &mut self.curl, dt, self.eps);

        diffuse(d, self.iters, 1, &mut self.u0, &self.u, self.visc, dt);
        diffuse(d, self.iters, 2, &mut self.v0, &self.v, self.visc, dt);
        project(d, self.iters, &mut self.u0, &mut self.v0, &mut self.u, &mut self.v);

        advect(d, 1, &mut self.u, &self.u0, &self.u0, &self.v0, dt);
        advect(d, 2, &mut self.v, &self.v0, &self.u0, &self.v0, dt);
        project(d, self.iters, &mut self.u, &mut self.v, &mut self.u0, &mut self.v0);
    }

    fn dens_step(&mut self, dt: f32) {
        let d = self.dims();
        add_source(&mut self.d, &self.d_src, dt);
        diffuse(d, self.iters, 0, &mut self.d0, &self.d, self.diff, dt);
        advect(d, 0, &mut self.d, &self.d0, &self.u, &self.v, dt);
        let f = self.fade;
        self.d.iter_mut().for_each(|x| *x *= f);
    }

    // ------------------------------------------------------------------
    // seeding / interaction
    // ------------------------------------------------------------------

    /// Inject a Gaussian rotational impulse (a vortex) plus a dye blob.
    pub fn splat_vortex(&mut self, cx: f32, cy: f32, strength: f32, radius: f32, dye: f32) {
        let r2 = radius * radius;
        let rad = (radius * 3.0).ceil() as i32;
        let ci = cx.round() as i32;
        let cj = cy.round() as i32;
        for dj in -rad..=rad {
            for di in -rad..=rad {
                let i = ci + di;
                let j = cj + dj;
                if i < 1 || i > self.nx as i32 || j < 1 || j > self.ny as i32 {
                    continue;
                }
                let fall = (-((di * di + dj * dj) as f32) / (2.0 * r2)).exp();
                let k = self.ix(i as usize, j as usize);
                self.u[k] += -(dj as f32) * strength * fall;
                self.v[k] += (di as f32) * strength * fall;
                self.d[k] += dye * fall;
            }
        }
    }

    /// Radial push away from the centre (negative strength = implode) + dye.
    pub fn splat_burst(&mut self, cx: f32, cy: f32, strength: f32, radius: f32, dye: f32) {
        let r2 = radius * radius;
        let rad = (radius * 3.0).ceil() as i32;
        let ci = cx.round() as i32;
        let cj = cy.round() as i32;
        for dj in -rad..=rad {
            for di in -rad..=rad {
                let i = ci + di;
                let j = cj + dj;
                if i < 1 || i > self.nx as i32 || j < 1 || j > self.ny as i32 {
                    continue;
                }
                let d2 = (di * di + dj * dj) as f32;
                let r = d2.sqrt().max(1.0);
                let fall = (-d2 / (2.0 * r2)).exp();
                let k = self.ix(i as usize, j as usize);
                self.u[k] += (di as f32 / r) * strength * fall;
                self.v[k] += (dj as f32 / r) * strength * fall;
                self.d[k] += dye * fall;
            }
        }
    }

    /// Directional push along (dx, dy) + dye.
    pub fn splat_jet(&mut self, cx: f32, cy: f32, dx: f32, dy: f32, strength: f32, radius: f32, dye: f32) {
        let (dx, dy) = norm_dir(dx, dy);
        let r2 = radius * radius;
        let rad = (radius * 3.0).ceil() as i32;
        let ci = cx.round() as i32;
        let cj = cy.round() as i32;
        for dj in -rad..=rad {
            for di in -rad..=rad {
                let i = ci + di;
                let j = cj + dj;
                if i < 1 || i > self.nx as i32 || j < 1 || j > self.ny as i32 {
                    continue;
                }
                let fall = (-((di * di + dj * dj) as f32) / (2.0 * r2)).exp();
                let k = self.ix(i as usize, j as usize);
                self.u[k] += dx * strength * fall;
                self.v[k] += dy * strength * fall;
                self.d[k] += dye * fall;
            }
        }
    }

    /// Two counter-rotating cores propel a ring along (dx, dy).
    pub fn splat_dipole(&mut self, cx: f32, cy: f32, dx: f32, dy: f32, strength: f32, radius: f32, dye: f32) {
        let (dx, dy) = norm_dir(dx, dy);
        let (px, py) = (-dy, dx);
        let off = radius * 0.6;
        self.splat_vortex(cx + px * off, cy + py * off, strength, radius * 0.8, dye * 0.55);
        self.splat_vortex(cx - px * off, cy - py * off, -strength, radius * 0.8, dye * 0.55);
        self.splat_jet(cx, cy, dx, dy, strength * 0.6, radius * 0.7, dye * 0.4);
    }

    /// Expanding ring: radial push concentrated in a Gaussian shell.
    pub fn splat_shock(&mut self, cx: f32, cy: f32, strength: f32, radius: f32, dye: f32) {
        let rad = (radius * 2.0).ceil() as i32;
        let shell = radius * 0.5;
        let ring = radius * 0.6;
        let ci = cx.round() as i32;
        let cj = cy.round() as i32;
        for dj in -rad..=rad {
            for di in -rad..=rad {
                let i = ci + di;
                let j = cj + dj;
                if i < 1 || i > self.nx as i32 || j < 1 || j > self.ny as i32 {
                    continue;
                }
                let r = ((di * di + dj * dj) as f32).sqrt().max(1.0);
                let fall = (-((r - ring) * (r - ring)) / (2.0 * shell * shell)).exp();
                let k = self.ix(i as usize, j as usize);
                self.u[k] += (di as f32 / r) * strength * fall;
                self.v[k] += (dj as f32 / r) * strength * fall;
                self.d[k] += dye * fall * 0.8;
            }
        }
    }

    /// A handful of small random vortices — chaotic churn. `rng` yields 0..1.
    pub fn splat_turb(&mut self, cx: f32, cy: f32, strength: f32, radius: f32, dye: f32, rng: &mut impl FnMut() -> f32) {
        for _ in 0..6 {
            let a = rng() * std::f32::consts::TAU;
            let rr = rng() * radius;
            let x = cx + a.cos() * rr;
            let y = cy + a.sin() * rr;
            let sign = if rng() < 0.5 { -1.0 } else { 1.0 };
            self.splat_vortex(x, y, sign * strength * (0.5 + rng()), radius * 0.35, dye * 0.5);
        }
    }

    /// Pure dye blob — no velocity.
    pub fn splat_bloom(&mut self, cx: f32, cy: f32, radius: f32, dye: f32) {
        let r2 = radius * radius;
        let rad = (radius * 3.0).ceil() as i32;
        let ci = cx.round() as i32;
        let cj = cy.round() as i32;
        for dj in -rad..=rad {
            for di in -rad..=rad {
                let i = ci + di;
                let j = cj + dj;
                if i < 1 || i > self.nx as i32 || j < 1 || j > self.ny as i32 {
                    continue;
                }
                let fall = (-((di * di + dj * dj) as f32) / (2.0 * r2)).exp();
                let k = self.ix(i as usize, j as usize);
                self.d[k] += dye * fall;
            }
        }
    }

    /// Opposite drift on either side of the axis -> the sheet rolls up.
    pub fn splat_shear(&mut self, cx: f32, cy: f32, dx: f32, dy: f32, strength: f32, radius: f32, dye: f32) {
        let (dx, dy) = norm_dir(dx, dy);
        let (px, py) = (-dy, dx);
        let r2 = radius * radius;
        let rad = (radius * 3.0).ceil() as i32;
        let ci = cx.round() as i32;
        let cj = cy.round() as i32;
        for dj in -rad..=rad {
            for di in -rad..=rad {
                let i = ci + di;
                let j = cj + dj;
                if i < 1 || i > self.nx as i32 || j < 1 || j > self.ny as i32 {
                    continue;
                }
                let fall = (-((di * di + dj * dj) as f32) / (2.0 * r2)).exp();
                let s = if di as f32 * px + dj as f32 * py >= 0.0 { 1.0 } else { -1.0 };
                let k = self.ix(i as usize, j as usize);
                self.u[k] += dx * strength * s * fall;
                self.v[k] += dy * strength * s * fall;
                self.d[k] += dye * fall;
            }
        }
    }

    /// Mobile-parity brush dispatch: turn a point + selected brush into the
    /// matching distortion. `spin` picks the vortex direction, `scale` shrinks
    /// autopilot swirls, and `rng` (0..1) supplies the random jet/dipole/shear
    /// heading plus turbulence jitter.
    pub fn apply_brush(&mut self, brush: Brush, cx: f32, cy: f32, spin: f32, scale: f32, rng: &mut impl FnMut() -> f32) {
        let rad = (self.nx.min(self.ny) as f32 * 0.07).clamp(3.0, 11.0) * scale;
        let s = 1.15 * scale;
        let dye = 1.2;
        let dir = |r: &mut dyn FnMut() -> f32| {
            let a = r() * std::f32::consts::TAU;
            (a.cos(), a.sin())
        };
        match brush {
            Brush::Vortex => self.splat_vortex(cx, cy, spin * s, rad, dye),
            Brush::Burst => self.splat_burst(cx, cy, s * 0.9, rad, dye),
            Brush::Implode => self.splat_burst(cx, cy, -s * 0.9, rad, dye * 0.6),
            Brush::Jet => {
                let (dx, dy) = dir(rng);
                self.splat_jet(cx, cy, dx, dy, s * 1.4, rad, dye);
            }
            Brush::Dipole => {
                let (dx, dy) = dir(rng);
                self.splat_dipole(cx, cy, dx, dy, s * 1.2, rad, dye);
            }
            Brush::Shock => self.splat_shock(cx, cy, s * 1.3, rad * 1.4, dye),
            Brush::Turbulence => self.splat_turb(cx, cy, s, rad * 1.3, dye * 0.8, rng),
            Brush::Bloom => self.splat_bloom(cx, cy, rad * 1.15, dye * 1.3),
            Brush::Shear => {
                let (dx, dy) = dir(rng);
                self.splat_shear(cx, cy, dx, dy, s * 1.1, rad * 1.2, dye);
            }
        }
    }

    /// Uniform body force over the interior (gravity / wind) — folded into the
    /// velocity sources for the next step.
    pub fn add_uniform(&mut self, ax: f32, ay: f32) {
        for j in 1..=self.ny {
            let b = j * self.w;
            for i in 1..=self.nx {
                self.u_src[b + i] += ax;
                self.v_src[b + i] += ay;
            }
        }
    }

    /// Add a directional force + dye over a small brush (used while dragging).
    pub fn add_force(&mut self, i: usize, j: usize, fx: f32, fy: f32, dye: f32, brush: i32) {
        let denom = 2.0 * (brush * brush) as f32 + 0.1;
        for dj in -brush..=brush {
            for di in -brush..=brush {
                let x = i as i32 + di;
                let y = j as i32 + dj;
                if x < 1 || x > self.nx as i32 || y < 1 || y > self.ny as i32 {
                    continue;
                }
                let fall = (-((di * di + dj * dj) as f32) / denom).exp();
                let k = self.ix(x as usize, y as usize);
                self.u_src[k] += fx * fall;
                self.v_src[k] += fy * fall;
                self.d_src[k] += dye * fall;
            }
        }
    }
}

/// Normalize a direction; a zero vector stays zero (mobile `hypot(..)||1`).
fn norm_dir(dx: f32, dy: f32) -> (f32, f32) {
    let l = (dx * dx + dy * dy).sqrt();
    let l = if l > 0.0 { l } else { 1.0 };
    (dx / l, dy / l)
}

// ======================================================================
// solver kernels — free functions over slices so the borrow checker can
// see that the grids passed in are disjoint fields of `Fluid`.
// ======================================================================

fn add_source(x: &mut [f32], s: &[f32], dt: f32) {
    for k in 0..x.len() {
        x[k] += dt * s[k];
    }
}

fn set_bnd(d: Dims, b: u8, x: &mut [f32]) {
    let (nx, ny) = (d.nx, d.ny);
    for j in 1..=ny {
        x[ix(d, 0, j)] = if b == 1 { -x[ix(d, 1, j)] } else { x[ix(d, 1, j)] };
        x[ix(d, nx + 1, j)] = if b == 1 { -x[ix(d, nx, j)] } else { x[ix(d, nx, j)] };
    }
    for i in 1..=nx {
        x[ix(d, i, 0)] = if b == 2 { -x[ix(d, i, 1)] } else { x[ix(d, i, 1)] };
        x[ix(d, i, ny + 1)] = if b == 2 { -x[ix(d, i, ny)] } else { x[ix(d, i, ny)] };
    }
    x[ix(d, 0, 0)] = 0.5 * (x[ix(d, 1, 0)] + x[ix(d, 0, 1)]);
    x[ix(d, 0, ny + 1)] = 0.5 * (x[ix(d, 1, ny + 1)] + x[ix(d, 0, ny)]);
    x[ix(d, nx + 1, 0)] = 0.5 * (x[ix(d, nx, 0)] + x[ix(d, nx + 1, 1)]);
    x[ix(d, nx + 1, ny + 1)] = 0.5 * (x[ix(d, nx, ny + 1)] + x[ix(d, nx + 1, ny)]);
}

fn lin_solve(d: Dims, iters: usize, b: u8, x: &mut [f32], x0: &[f32], a: f32, c: f32) {
    let inv_c = 1.0 / c;
    let w = d.w;
    for _ in 0..iters {
        for j in 1..=d.ny {
            for i in 1..=d.nx {
                let k = ix(d, i, j);
                x[k] = (x0[k] + a * (x[k - 1] + x[k + 1] + x[k - w] + x[k + w])) * inv_c;
            }
        }
        set_bnd(d, b, x);
    }
}

fn diffuse(d: Dims, iters: usize, b: u8, dst: &mut [f32], src: &[f32], diff: f32, dt: f32) {
    let a = dt * diff * d.nx as f32 * d.ny as f32;
    lin_solve(d, iters, b, dst, src, a, 1.0 + 4.0 * a);
}

fn advect(d: Dims, b: u8, dst: &mut [f32], src: &[f32], vu: &[f32], vv: &[f32], dt: f32) {
    let (nx, ny) = (d.nx, d.ny);
    let dt0x = dt * nx as f32;
    let dt0y = dt * ny as f32;
    for j in 1..=ny {
        for i in 1..=nx {
            let k = ix(d, i, j);
            let mut x = i as f32 - dt0x * vu[k];
            let mut y = j as f32 - dt0y * vv[k];
            x = x.clamp(0.5, nx as f32 + 0.5);
            y = y.clamp(0.5, ny as f32 + 0.5);
            let i0 = x as usize;
            let i1 = i0 + 1;
            let j0 = y as usize;
            let j1 = j0 + 1;
            let s1 = x - i0 as f32;
            let s0 = 1.0 - s1;
            let t1 = y - j0 as f32;
            let t0 = 1.0 - t1;
            dst[k] = s0 * (t0 * src[ix(d, i0, j0)] + t1 * src[ix(d, i0, j1)])
                + s1 * (t0 * src[ix(d, i1, j0)] + t1 * src[ix(d, i1, j1)]);
        }
    }
    set_bnd(d, b, dst);
}

fn project(d: Dims, iters: usize, vu: &mut [f32], vv: &mut [f32], p: &mut [f32], div: &mut [f32]) {
    let n = d.nx as f32;
    let w = d.w;
    for j in 1..=d.ny {
        for i in 1..=d.nx {
            let k = ix(d, i, j);
            div[k] = -0.5 * (vu[k + 1] - vu[k - 1] + vv[k + w] - vv[k - w]) / n;
            p[k] = 0.0;
        }
    }
    set_bnd(d, 0, div);
    set_bnd(d, 0, p);
    lin_solve(d, iters, 0, p, div, 1.0, 4.0);
    for j in 1..=d.ny {
        for i in 1..=d.nx {
            let k = ix(d, i, j);
            vu[k] -= 0.5 * n * (p[k + 1] - p[k - 1]);
            vv[k] -= 0.5 * n * (p[k + w] - p[k - w]);
        }
    }
    set_bnd(d, 1, vu);
    set_bnd(d, 2, vv);
}

fn compute_curl(d: Dims, u: &[f32], v: &[f32], curl: &mut [f32]) {
    let w = d.w;
    for j in 1..=d.ny {
        for i in 1..=d.nx {
            let k = ix(d, i, j);
            curl[k] = 0.5 * ((v[k + 1] - v[k - 1]) - (u[k + w] - u[k - w]));
        }
    }
}

fn vort_confine(d: Dims, u: &mut [f32], v: &mut [f32], curl: &mut [f32], dt: f32, eps: f32) {
    if eps <= 0.0 {
        return;
    }
    let w = d.w;
    // curl of the velocity field (z component)
    for j in 1..=d.ny {
        for i in 1..=d.nx {
            let k = ix(d, i, j);
            curl[k] = 0.5 * ((v[k + 1] - v[k - 1]) - (u[k + w] - u[k - w]));
        }
    }
    // force = eps * (N x omega), N = grad|curl| normalized
    for j in 2..d.ny {
        for i in 2..d.nx {
            let k = ix(d, i, j);
            let mut n_x = 0.5 * (curl[k + 1].abs() - curl[k - 1].abs());
            let mut n_y = 0.5 * (curl[k + w].abs() - curl[k - w].abs());
            let len = (n_x * n_x + n_y * n_y).sqrt() + 1e-5;
            n_x /= len;
            n_y /= len;
            let wz = curl[k];
            u[k] += dt * eps * (n_y * wz);
            v[k] += dt * eps * (-n_x * wz);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stays_finite_and_bounded() {
        for &eps in &[0.0f32, 12.0] {
            let mut f = Fluid::new(120, 60);
            f.eps = eps;
            for (x, y, s) in [(30.0, 30.0, 1.0), (60.0, 30.0, -1.0), (90.0, 30.0, 1.0), (45.0, 45.0, -1.0)] {
                f.splat_vortex(x, y, s * 0.9, 6.0, 1.0);
            }
            let mut seed = 0x1234_5678u32;
            let mut rng = || {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed >> 8) as f32 / (1u32 << 24) as f32
            };
            for t in 0..800 {
                if t % 50 == 0 {
                    let i = (10.0 + rng() * 100.0) as usize;
                    let j = (10.0 + rng() * 40.0) as usize;
                    f.add_force(i, j, (rng() * 2.0 - 1.0) * 30.0, (rng() * 2.0 - 1.0) * 30.0, 15.0, 2);
                }
                f.step(0.12);
            }
            let max_u = f.u.iter().cloned().fold(0.0f32, |m, x| m.max(x.abs()));
            let max_curl = f.curl.iter().cloned().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(f.u.iter().all(|x| x.is_finite()), "velocity went non-finite");
            assert!(f.d.iter().all(|x| x.is_finite()), "dye went non-finite");
            assert!(max_u < 50.0, "velocity blew up: {max_u}");
            assert!(max_curl > 0.0, "curl field not populated (eps={eps})");
        }
    }

    #[test]
    fn brushes_and_gravity_stay_finite() {
        let mut seed = 0xC0FF_EE11u32;
        let mut rng = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        let mut f = Fluid::new(90, 50);
        for (n, &b) in BRUSHES.iter().enumerate() {
            let x = 8.0 + n as f32 * 9.0;
            let spin = if n % 2 == 0 { 1.0 } else { -1.0 };
            f.apply_brush(b, x, 25.0, spin, 1.0, &mut rng);
        }
        for _ in 0..300 {
            f.add_uniform(0.0, 2.0); // gravity at the default slider setting
            f.step(0.12);
        }
        let max_u = f.u.iter().cloned().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(f.u.iter().all(|x| x.is_finite()), "velocity went non-finite");
        assert!(f.v.iter().all(|x| x.is_finite()), "velocity went non-finite");
        assert!(f.d.iter().all(|x| x.is_finite()), "dye went non-finite");
        assert!(max_u < 50.0, "velocity blew up: {max_u}");
    }
}
