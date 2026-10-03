# ascii fluid // voyd

A slow-motion vortex fluid simulation rendered as ASCII art, in a resizable
pop-out window. Native desktop port of the browser version — same solver, no
terminal spam, drawn straight to a pixel framebuffer.

The fluid is a Jos Stam *stable-fluids* solver (semi-Lagrangian advection +
Gauss–Seidel pressure projection) with Fedkiw **vorticity confinement**, which
is what keeps the vortices crisp and orbiting each other in slow motion instead
of numerically diffusing away after a few seconds.

Feature parity with the mobile app (`ascii_fluid_mobile`): **9 distortion
brushes**, **42 palettes** (signature / classic / retro computing / retro
gaming / 80s neon, with hard colour banding on the limited-palette ones),
**13 charsets** (including braille, box drawing, arrows, stars, and hiragana,
all drawn with the embedded 8×8 bitmap font), a **spin-direction lock**,
**gravity** (a fixed downward body force — no tilt sensor on a desktop),
**randomize**, and **settings persistence**. Deliberately not ported:
multi-touch, device tilt, and web-font selection (the renderer is a fixed
bitmap font); katakana / kanji / math / runic charsets (no 8×8 glyph
coverage).

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
| click | inject the selected brush (vortex spin alternates, unless locked) |
| shift + click | same, with reversed spin |
| drag | push the fluid and lay down ink |

**Keyboard**
| key | effect |
|---|---|
| `Space` | pause / resume |
| `R` | reseed |
| `Shift+R` | randomize palette + charset + brush, then reseed |
| `C` | clear the dye |
| `F` | cycle field: dye → speed → vorticity |
| `T` / `Shift+T` | next / previous palette (42) |
| `G` / `Shift+G` | next / previous charset (13) |
| `B` / `Shift+B` | next / previous brush (9) |
| `D` | spin direction: auto → CW → CCW |
| `W` | toggle gravity (strength = the gravity parameter) |
| `I` | invert |
| `S` | color-by-spin (vorticity view only) |
| `A` | toggle autopilot |
| `H` | hide / show the HUD |
| `Tab` / `Shift+Tab` | select next / previous parameter |
| `←` `→` (or `[` `]`, `-` `=`) | nudge the selected parameter |
| `Q` / `Esc` | quit |

The selected parameter and its value show in the HUD strip along the bottom,
tinted to match the active palette. Adjustable parameters: speed, vorticity,
viscosity, dye spread, persistence, brush force, gravity, contrast, detail
(cell size), and seeds.

**Brushes** — what a click (and reseeding, and the autopilot) injects:
vortex, burst, implode, jet, dipole, shockwave, turbulence, ink bloom, shear.

To get the signature look: turn **vorticity** up and **speed** down for slow,
sinewy motion; raise **persistence** so ink trails linger; switch the field to
**vorticity** + **color-by-spin** to see clockwise / counter-clockwise cores as
warm / cool. **Autopilot** (on by default) feeds in faint swirls so it runs as a
generative background unattended. Try the **shockwave** brush on the *Teletext*
palette with the *braille* charset for maximum terminal-archaeology energy.

The initial scene and the autopilot sequence are driven by a fixed-seed RNG, so
the unattended background is reproducible run to run.

Settings persist across runs to `$XDG_CONFIG_HOME/ascii_fluid/config`
(fallback `~/.config/ascii_fluid/config`) — a plain `key=value` file written on
quit. Delete it to reset. Pause and gravity are intentionally not remembered.

## Layout

```
src/
  main.rs     window, main loop, fps pacing, resize handling
  app.rs      state, parameters, input mapping, autopilot, HUD, config file
  fluid.rs    the solver (stable-fluids + vorticity confinement + brushes)
  render.rs   field sampling, auto-exposure, ASCII glyph blitting
  palette.rs  palettes -> color LUTs, and the ASCII ramps
```

`cargo test` runs a stability soak on the solver (all brushes + gravity,
asserting the fields stay finite and bounded), checks every ramp character
resolves to a non-blank glyph, and round-trips the config file.
`cargo run --release -- fonttest` dumps per-ramp glyph coverage and a sample of
glyphs as ASCII art without opening a window.
