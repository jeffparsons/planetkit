use piston_window::PistonWindow;
use piston::input::{ RenderArgs, UpdateArgs };
use slog::Logger;
use vecmath;
use gfx_device_gl;
use camera_controllers;

use globe;

fn get_projection(w: &PistonWindow) -> [[f32; 4]; 4] {
    use piston::window::Window;
    use camera_controllers::CameraPerspective;

    let draw_size = w.window.draw_size();
    CameraPerspective {
        fov: 90.0, near_clip: 0.01, far_clip: 100.0,
        aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32)
    }.projection()
}

pub struct App {
    t: f64,
    log: Logger,
    draw: globe::Draw<gfx_device_gl::Resources>,
    model: vecmath::Matrix4<f32>,
    projection: [[f32; 4]; 4],
    first_person: camera_controllers::FirstPerson,
}

impl App {
    pub fn new(parent_log: &Logger, window: &PistonWindow) -> App {
        use camera_controllers::{
            FirstPersonSettings,
            FirstPerson,
        };

        // Make OpenGL resource factory.
        // We'll use this for creating all our vertex buffers, etc.
        let factory = &mut window.factory.clone();

        // Rendering system.
        let mut draw = globe::Draw::new(factory);

        let log = parent_log.new(o!());

        // Make globe and create a mesh for each of its chunks.
        // TODO: move this out of app
        let globe = globe::Globe::new_example(&log);
        let globe_view = globe::View::new(&globe, &log);
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

        let model: vecmath::Matrix4<f32> = vecmath::mat4_id();
        let projection = get_projection(&window);
        let first_person = FirstPerson::new(
            [0.5, 0.5, 4.0],
            FirstPersonSettings::keyboard_wasd()
        );

        App {
            t: 0.0,
            log: log,
            draw: draw,
            model: model,
            projection: projection,
            first_person: first_person,
        }
    }

    pub fn run(&mut self, window: &mut PistonWindow) {
        use piston::input::*;
        use piston::event_loop::Events;

        info!(self.log, "Starting event loop");

        let mut events = window.events();
        while let Some(e) = events.next(window) {
            self.first_person.event(&e);

            window.draw_3d(&e, |mut window| {
                let args = e.render_args().unwrap();
                self.render(&args, &mut window);
            });

            if let Some(_) = e.resize_args() {
                self.projection = get_projection(&window);
            }

            if let Some(u) = e.update_args() {
                self.update(&u);
            }
        }

        info!(self.log, "Quitting");
    }

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
