extern crate chrono;
extern crate rand;
extern crate noise;
extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate piston_window;
extern crate camera_controllers;
extern crate vecmath;
extern crate shader_version;
extern crate nalgebra as na;
#[macro_use]
extern crate slog;
extern crate slog_term;

mod globe;
mod types;
mod app;
mod window;

fn main() {
    // Set up logger.
    use slog::DrainExt;
    let drain = slog_term::streamer().compact().build().fuse();
    let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));
    let log = root_log;

    let mut window = window::make_window(&log);
    let mut app = app::App::new(&log, &window);
    app.run(&mut window);
}
