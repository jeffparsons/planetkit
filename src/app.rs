use piston_window::PistonWindow;
use piston::input::{ RenderArgs, UpdateArgs };
use slog::Logger;
use vecmath;
use gfx_device_gl;
use camera_controllers;
use specs;

use render_sys;
use types::*;

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
    t: TimeDelta,
    log: Logger,
    planner: specs::Planner<TimeDelta>,
    render_sys: render_sys::Draw<gfx_device_gl::Resources>,
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

        // Create SPECS world and, system execution planner
        // for it with two threads.
        //
        // This manages execution of all game systems,
        // i.e. the interaction between sets of components.
        let world = specs::World::new();
        let planner = specs::Planner::new(world, 2);

        // Rendering system.
        let render_sys = render_sys::Draw::new(factory);

        let log = parent_log.new(o!());

        let model: vecmath::Matrix4<f32> = vecmath::mat4_id();
        let projection = get_projection(&window);
        let first_person = FirstPerson::new(
            [0.5, 0.5, 4.0],
            FirstPersonSettings::keyboard_wasd()
        );

        App {
            t: 0.0,
            log: log,
            planner: planner,
            render_sys: render_sys,
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

    pub fn render_sys(&mut self) -> &mut render_sys::Draw<gfx_device_gl::Resources> {
        &mut self.render_sys
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
        self.render_sys.draw(
            &mut window.encoder,
            model_view_projection,
        );
    }

    fn update(&mut self, args: &UpdateArgs) {
        self.t += args.dt;
        self.planner.dispatch(args.dt);
    }
}
