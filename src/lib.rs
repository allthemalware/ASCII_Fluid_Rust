//! PyO3 bindings — expose the stable-fluids solver to Python so Lore's effects layer
//! (Lore/fluidsim.py `RustFluid`) can run it **live** as a backend.
//!
//! Built as the `ascii_fluid` extension module (`maturin develop --release`). The
//! standalone window binary (main.rs) is untouched — it keeps its own `mod fluid`.
//!
//! Contract Lore expects: `Sim(w, h, mode)` with `.step(dt)` and `.field() -> bytes`
//! (h*w of 0..255, row-major). `mode` is "fire" (hot dye + upward jet at the base) or
//! anything else = "smoke"/corruption (self-seeding vortices). `add_source` lets the
//! game push emitters at events (a spell impact, an ignition).

mod fluid;

use fluid::Fluid;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Small xorshift RNG so the sim can self-seed without pulling a crate.
struct Rng(u32);
impl Rng {
    fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

#[pyclass]
struct Sim {
    f: Fluid,
    mode: String,
    gain: f32,
    rng: Rng,
}

#[pymethods]
impl Sim {
    #[new]
    fn new(w: usize, h: usize, mode: String) -> Self {
        let mut f = Fluid::new(w.max(8), h.max(8));
        let gain = if mode == "fire" {
            f.eps = 16.0;    // swirl → flame licks rather than a uniform plume
            f.fade = 0.94;   // cool as it climbs, but persist long enough to rise
            f.visc = 1e-5;
            1.0
        } else {
            f.eps = 16.0;    // crisp vortices for corruption ooze
            f.fade = 0.95;   // churn (don't accumulate to a solid fill)
            1.0
        };
        Sim { f, mode, gain, rng: Rng(0x9E37_79B9) }
    }

    /// Advance one step — self-seed the mode's sources first so the field stays alive.
    fn step(&mut self, dt: f32) {
        let (nx, ny) = (self.f.nx, self.f.ny);
        if self.mode == "fire" {
            let base = ny.saturating_sub(2).max(1);          // base = high j = bottom of the billboard
            let cx = nx as f32 * (0.40 + 0.20 * self.rng.f());   // a flickering centre plume
            let i = (cx as usize).clamp(1, nx);
            self.f.add_force(i, base, (self.rng.f() - 0.5) * 6.0, -14.0, 22.0, 2);
            if self.rng.f() < 0.6 {                          // a secondary lick for liveliness
                let i2 = ((cx + (self.rng.f() - 0.5) * 6.0) as usize).clamp(1, nx);
                self.f.add_force(i2, base, 0.0, -11.0, 12.0, 2);
            }
        } else if self.rng.f() < 0.5 {                       // occasional vortices keep corruption churning
            let cx = 2.0 + self.rng.f() * (nx as f32 - 4.0);
            let cy = 2.0 + self.rng.f() * (ny as f32 - 4.0);
            let s = (self.rng.f() * 2.0 - 1.0) * 1.2;
            self.f.splat_vortex(cx, cy, s, 3.0 + self.rng.f() * 3.0, 0.6);
        }
        // The solver's advection scales by dt*nx, so it needs a small fixed step (CFL);
        // decouple it from Lore's per-frame dt — one Lore tick = one solver sub-step.
        let _ = dt;
        self.f.step(0.11);
    }

    /// The dye field as h*w bytes (0..255, row-major) — what Lore reads each frame.
    fn field<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let (nx, ny) = (self.f.nx, self.f.ny);
        let mut buf = vec![0u8; nx * ny];
        for j in 0..ny {
            for i in 0..nx {
                let v = self.f.dye(i + 1, j + 1) * self.gain;   // interior is 1..=n
                buf[j * nx + i] = (v.clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
        PyBytes::new(py, &buf)
    }

    /// Inject dye + a vortex at a point (cell coords) — for effects that react to the
    /// game: a spell's impact, something catching fire, corruption spreading.
    fn add_source(&mut self, x: f32, y: f32, r: f32, amount: f32) {
        self.f.splat_vortex(
            x.clamp(1.0, self.f.nx as f32),
            y.clamp(1.0, self.f.ny as f32),
            0.6,
            r.max(1.0),
            amount,
        );
    }

    #[getter]
    fn w(&self) -> usize { self.f.nx }
    #[getter]
    fn h(&self) -> usize { self.f.ny }
}

#[pymodule]
fn ascii_fluid(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Sim>()?;
    Ok(())
}
