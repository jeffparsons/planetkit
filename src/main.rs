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
extern crate specs;

mod globe;
mod types;
mod app;
mod window;
mod render_sys;

fn main() {
    // Set up logger.
    use slog::DrainExt;
    let drain = slog_term::streamer().compact().build().fuse();
    let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));
    let log = root_log;

    let mut window = window::make_window(&log);
    let mut app = app::App::new(&log, &window);

    // Make globe and create a mesh for each of its chunks.
    //
    // TODO: move the geometry generation bits somewhere else;
    // the user shouldn't have to mess with any of this.
    let factory = &mut window.factory.clone();
    let globe = globe::Globe::new_example(&log);
    let globe_view = globe::View::new(&globe, &log);
    let geometry = globe_view.make_geometry(&globe);
    for (vertices, vertex_indices) in geometry {
        let mesh = render_sys::Mesh::new(
            factory,
            vertices,
            vertex_indices,
            window.output_color.clone(),
            window.output_stencil.clone(),
        );
        app.render_sys().add_mesh(mesh);
    }

    app.run(&mut window);
}
