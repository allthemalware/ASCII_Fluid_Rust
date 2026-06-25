# ascii fluid // voyd

A slow-motion vortex fluid simulation rendered as ASCII art, in a resizable
pop-out window. Native desktop port of the browser version — same solver, no
terminal spam, drawn straight to a pixel framebuffer.

The fluid is a Jos Stam *stable-fluids* solver (semi-Lagrangian advection +
Gauss–Seidel pressure projection) with Fedkiw **vorticity confinement**, which
is what keeps the vortices crisp and orbiting each other in slow motion instead
of numerically diffusing away after a few seconds.

## Run

```sh
cargo run --release
```

First build pulls two small crates (`minifb` for the window, `font8x8` for the
glyphs) and takes a few seconds. `--release` matters — the solver is number-
crunching every frame.

### Requirements
- A recent Rust toolchain (the default manifest uses crates that need a 2024-
  edition-aware cargo; any current stable is fine).
- Linux (X11 or Wayland — `minifb` dlopens the system libs), macOS, or Windows.
- On older toolchains that reject `edition2024` transitive deps, force the X11
  backend only:

  ```toml
  minifb = { version = "0.27", default-features = false, features = ["x11"] }
  ```

## Controls

**Mouse**
| action | effect |
|---|---|
| click | spawn a vortex (spin alternates each click) |
| shift + click | spawn with reversed spin |
| drag | push the fluid and lay down ink |

**Keyboard**
| key | effect |
|---|---|
| `Space` | pause / resume |
| `R` | reseed the vortices |
| `C` | clear the dye |
| `F` | cycle field: dye → speed → vorticity |
| `T` | cycle palette |
| `G` | cycle charset / ramp |
| `I` | invert |
| `S` | color-by-spin (vorticity view only) |
| `A` | toggle autopilot |
| `H` | hide / show the HUD |
| `Tab` / `Shift+Tab` | select next / previous parameter |
| `←` `→` (or `[` `]`, `-` `=`) | nudge the selected parameter |
| `Q` / `Esc` | quit |

The selected parameter and its value show in the HUD strip along the bottom.
Adjustable parameters: speed, vorticity, viscosity, dye spread, persistence,
brush force, contrast, detail (cell size), and vortex count.

To get the signature look: turn **vorticity** up and **speed** down for slow,
sinewy motion; raise **persistence** so ink trails linger; switch the field to
**vorticity** + **color-by-spin** to see clockwise / counter-clockwise cores as
warm / cool. **Autopilot** (on by default) feeds in faint swirls so it runs as a
generative background unattended.

The initial scene and the autopilot sequence are driven by a fixed-seed RNG, so
the unattended background is reproducible run to run.

## Layout

```
src/
  main.rs     window, main loop, fps pacing, resize handling
  app.rs      state, parameters, input mapping, autopilot, HUD
  fluid.rs    the solver (stable-fluids + vorticity confinement)
  render.rs   field sampling, auto-exposure, ASCII glyph blitting
  palette.rs  palettes -> color LUTs, and the ASCII ramps
```

`cargo test` runs a stability soak on the solver (asserts the fields stay finite
and bounded, with the curl field populated, for both confinement on and off).
