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

// TODO: move most of these into specific functions
use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use opengl_graphics::OpenGL;
use piston_window::PistonWindow;

mod globe;
mod types;

pub struct App {
    t: f64,
    draw: globe::Draw<gfx_device_gl::Resources>,
    model: vecmath::Matrix4<f32>,
    projection: [[f32; 4]; 4],
    first_person: camera_controllers::FirstPerson,
}

impl App {
    fn render(&mut self, args: &RenderArgs, window: &mut PistonWindow) {
        const CLEAR_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];
        window.encoder.clear(&window.output_color, CLEAR_COLOR);
        window.encoder.clear_depth(&window.output_stencil, 1.0);

        let model_view_projection = camera_controllers::model_view_projection(
            self.model,
            self.first_person.camera(args.ext_dt).orthogonal(),
            self.projection
        );

        // Draw the globe.
        self.draw.draw(
            &mut window.encoder,
            model_view_projection,
        );
    }

    fn update(&mut self, args: &UpdateArgs) {
        self.t += args.dt;
    }
}

fn main() {
    use piston::window::Window;
    use piston::window::AdvancedWindow;
    use camera_controllers::{
        FirstPersonSettings,
        FirstPerson,
        CameraPerspective,
    };

    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // Create an Glutin window.
    let mut window: PistonWindow = WindowSettings::new(
        "planetkit",
        [800, 600]
    )
        .opengl(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();
    window.set_capture_cursor(false);

    // Make OpenGL resource factory.
    // We'll use this for creating all our vertex buffers, etc.
    let factory = &mut window.factory.clone();

    // Rendering system.
    let mut draw = globe::Draw::new(
        factory,
    );

    // Make globe and create a mesh for each of its chunks.
    let globe = globe::Globe::new_example();
    let globe_view = globe::View::new(&globe);
    let geometry = globe_view.make_geometry(&globe);
    for (vertices, vertex_indices) in geometry {
        let mesh = globe::Mesh::new(
            factory,
            vertices,
            vertex_indices,
            window.output_color.clone(),
            window.output_stencil.clone(),
        );
        draw.add_mesh(mesh);
    }

    let get_projection = |w: &PistonWindow| {
        let draw_size = w.window.draw_size();
        CameraPerspective {
            fov: 90.0, near_clip: 0.01, far_clip: 100.0,
            aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32)
        }.projection()
    };

    let model: vecmath::Matrix4<f32> = vecmath::mat4_id();
    let projection = get_projection(&window);
    let first_person = FirstPerson::new(
        [0.5, 0.5, 4.0],
        FirstPersonSettings::keyboard_wasd()
    );

    let mut app = App {
        t: 0.0,
        draw: draw,
        model: model,
        projection: projection,
        first_person: first_person,
    };

    let mut events = window.events();
    while let Some(e) = events.next(&mut window) {
        app.first_person.event(&e);

        window.draw_3d(&e, |mut window| {
            let args = e.render_args().unwrap();
            app.render(&args, &mut window);
        });

        if let Some(_) = e.resize_args() {
            app.projection = get_projection(&window);
        }

        if let Some(u) = e.update_args() {
            app.update(&u);
        }
    }
}
