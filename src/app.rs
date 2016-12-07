use std::sync::mpsc;
use piston_window::PistonWindow;
use piston::input::{ UpdateArgs, RenderArgs };
use slog::Logger;
use vecmath;
use gfx_device_gl;
use camera_controllers;
use specs;

use render;
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
    render_sys: render::Draw<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    encoder_channel: render::EncoderChannel<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
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

        // Rendering system, with bi-directional channel to pass
        // encoder back and forth between this thread (which owns
        // the graphics device) and any number of game threads managed by Specs.
        let (render_sys_send, device_recv) = mpsc::channel();
        let (device_send, render_sys_recv) = mpsc::channel();
        let render_sys_encoder_channel = render::EncoderChannel {
            sender: render_sys_send,
            receiver: render_sys_recv,
        };
        let device_encoder_channel = render::EncoderChannel {
            sender: device_send,
            receiver: device_recv,
        };

        // Shove two encoders into the channel circuit.
        // This gives us "double-buffering" by having two encoders in flight.
        // This way the render system will always be able to populate
        // an encoder, even while this thread is busy flushing one
        // to the video card.
        //
        // (Note: this is separate from the double-buffering of the
        // output buffers -- this is the command buffer that we can fill
        // up with drawing commands _before_ flushing the whole thing to
        // the video card in one go.)
        let enc1 = window.encoder.clone_empty();
        let enc2 = window.encoder.clone_empty();
        // TODO: this carefully sending one encoder to each
        // channel is only because I'm temporarily calling
        // the rendering system synchronously until I get
        // around to turning it into a Specs system. Juggling like
        // this prevents deadlock.
        render_sys_encoder_channel.sender.send(enc1).unwrap();
        device_encoder_channel.sender.send(enc2).unwrap();

        let render_sys = render::Draw::new(
            factory,
            render_sys_encoder_channel,
            window.output_color.clone(),
            window.output_stencil.clone(),
        );

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
            encoder_channel: device_encoder_channel,
            model: model,
            projection: projection,
            first_person: first_person,
        }
    }

    pub fn run(&mut self, mut window: &mut PistonWindow) {
        use piston::input::*;
        use piston::event_loop::Events;

        info!(self.log, "Starting event loop");

        let mut events = window.events();
        while let Some(e) = events.next(window) {
            self.first_person.event(&e);

            if let Some(r) = e.render_args() {
                self.render(&r, &mut window);
            }

            if let Some(_) = e.resize_args() {
                self.projection = get_projection(&window);
            }

            if let Some(u) = e.update_args() {
                self.update(&u);
            }
        }

        info!(self.log, "Quitting");
    }

    pub fn render_sys(&mut self) -> &mut render::Draw<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer> {
        &mut self.render_sys
    }

    fn render(&mut self, args: &RenderArgs, window: &mut PistonWindow) {
        let mut encoder = self.encoder_channel.receiver.recv().expect("Render system hung up. That wasn't supposed to happen!");

        // TODO: what's make_current actually necessary for?
        // Do I even need to do this? (Ripped off `draw_3d`.)
        use piston::window::OpenGLWindow;
        window.window.make_current();

        // TODO: move into render system
        // The only thing we're using the render args for here
        // is the "extrapolated time", and I'm guessing there's
        // a far better way to go about that. (I.e. track it in
        // the render system.)
        let model_view_projection = camera_controllers::model_view_projection(
            self.model,
            self.first_person.camera(args.ext_dt).orthogonal(),
            self.projection
        );

        // Draw the globe.
        // TODO: move whole thing into render system.
        self.render_sys.draw(
            model_view_projection,
        );

        encoder.flush(&mut window.device);

        self.encoder_channel.sender.send(encoder).unwrap();
    }

    fn update(&mut self, args: &UpdateArgs) {
        self.t += args.dt;
        self.planner.dispatch(args.dt);
    }
}
