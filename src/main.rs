//! ascii fluid // voyd — slow-motion vortex fluid sim rendered as ASCII
//! in a resizable pop-out window.

mod app;
mod fluid;
mod palette;
mod render;

use app::App;
use minifb::{Scale, Window, WindowOptions};
use std::time::Instant;

const INIT_W: usize = 1100;
const INIT_H: usize = 680;

fn main() {
    // hidden debug path: `cargo run -- fonttest` prints a couple of glyphs so
    // we can confirm the 8x8 bit orientation without opening a window.
    if std::env::args().nth(1).as_deref() == Some("fonttest") {
        font_test();
        return;
    }

    let mut window = Window::new(
        "ascii fluid // voyd",
        INIT_W,
        INIT_H,
        WindowOptions {
            resize: true,
            scale: Scale::X1,
            ..WindowOptions::default()
        },
    )
    .expect("failed to open window");

    // ~36 fps so motion is independent of monitor refresh rate.
    window.set_target_fps(36);

    let mut app = App::new(INIT_W, INIT_H);
    let (mut w, mut h) = (INIT_W, INIT_H);
    let mut buf = vec![0u32; w * h];

    let mut last = Instant::now();
    let mut fps = 36.0f32;

    while window.is_open() && !app.quit {
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.1);
        last = now;
        if dt > 0.0 {
            // exponential moving average for a steady HUD readout
            fps += (1.0 / dt - fps) * 0.1;
        }

        // react to window resizing
        let (nw, nh) = window.get_size();
        if (nw, nh) != (w, h) && nw > 0 && nh > 0 {
            w = nw;
            h = nh;
            buf = vec![0u32; w * h];
            app.set_window_size(w, h);
        }

        app.handle_input(&window);
        app.autopilot(dt);
        app.step();
        app.render(&mut buf, w, h, fps);

        window
            .update_with_buffer(&buf, w, h)
            .expect("failed to update window");
    }

    app.save_config();
}

/// Dump glyph coverage for every ramp (flagging blanks), then draw a sample of
/// glyphs as ASCII art — confirms bit orientation and charset coverage without
/// opening a window.
fn font_test() {
    for ramp in palette::RAMPS {
        let missing: Vec<char> = ramp
            .chars
            .chars()
            .skip(1) // the blank slot
            .filter(|&c| !matches!(render::glyph(c), Some(g) if g.iter().any(|&r| r != 0)))
            .collect();
        if missing.is_empty() {
            println!("{:12} ok ({} glyphs)", ramp.name, ramp.chars.chars().count());
        } else {
            println!("{:12} MISSING {missing:?}", ramp.name);
        }
    }
    for ch in ['A', '#', '\u{2022}', '\u{25cf}', '\u{2191}', '\u{2192}', '\u{21d2}', '\u{27a4}', '\u{2726}', '\u{2605}', '\u{274b}', '\u{281f}', '\u{2591}', '\u{2500}', '\u{3042}'] {
        println!("glyph '{ch}':");
        let g = render::glyph(ch).unwrap_or([0; 8]);
        for row in g {
            let mut line = String::new();
            for bit in 0..8 {
                line.push(if row & (1 << bit) != 0 { '#' } else { '.' });
            }
            println!("  {line}");
        }
    }
}
