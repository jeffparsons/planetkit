extern crate noise;
extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;

use noise::{Brownian4, Seed};

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, OpenGL };

pub struct App {
    gl: GlGraphics,
    t: f64,
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        // TODO: is this wasteful at all?
        // I.e. should I be storing a PermutationTable or `noise` or something?
        let seed = Seed::new(12);
        let noise = Brownian4::new(noise::perlin4, 4).wavelength(1.0);

        // For now just drawing one coloured square
        // based on the current time.
        //
        // TODO: draw a grid of squares based on position and time.
        let val = noise.apply(&seed, &[42.0, 37.0, 2.0, self.t as f32]);

        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        let color: [f32; 4] = [val, val, val, 1.0];

        let square = rectangle::square(0.0, 0.0, 50.0);
        let x = (args.width / 2) as f64;
        let y = (args.height / 2) as f64;
        self.gl.draw(args.viewport(), |c, gl| {
            clear(GREEN, gl);
            let transform = c.transform.trans(x, y)
               .trans(-25.0, -25.0);
            rectangle(color, square, transform, gl);
        });
    }

    fn update(&mut self, args: &UpdateArgs) {
        self.t += args.dt;
    }
}

fn main() {
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // Create an Glutin window.
    let mut window: Window = WindowSettings::new(
            "black-triangle",
            [200, 200]
        )
        .opengl(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();

    let mut app = App {
        gl: GlGraphics::new(opengl),
        t: 0.0,
    };

    let mut events = window.events();
    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&r);
        }

        if let Some(u) = e.update_args() {
            app.update(&u);
        }
    }
}
