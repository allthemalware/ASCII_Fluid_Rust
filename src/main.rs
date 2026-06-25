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
}

fn font_test() {
    use font8x8::legacy::BASIC_LEGACY;
    for ch in ['A', 'F', 'R', '#'] {
        println!("glyph '{ch}':");
        let g = BASIC_LEGACY[ch as usize];
        for row in g {
            let mut line = String::new();
            for bit in 0..8 {
                line.push(if row & (1 << bit) != 0 { '#' } else { '.' });
            }
            println!("  {line}");
        }
    }
}
