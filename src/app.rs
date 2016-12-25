use std::sync::{ Arc, Mutex, mpsc };
use piston_window::PistonWindow;
use piston::input::{ UpdateArgs, RenderArgs };
use slog::Logger;
use gfx_device_gl;
use camera_controllers;
use specs;

use render;
use types::*;
use globe;
use cell_dweller;

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
    encoder_channel: render::EncoderChannel<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    input_sender: mpsc::Sender<cell_dweller::ControlEvent>,
    // TEMP: Share with rendering system until the rendering system
    // is smart enough to take full ownership of it.
    projection: Arc<Mutex<[[f32; 4]; 4]>>,
    first_person: Arc<Mutex<camera_controllers::FirstPerson>>,
}

impl App {
    pub fn new(parent_log: &Logger, window: &PistonWindow) -> App {
        use camera_controllers::{
            FirstPersonSettings,
            FirstPerson,
        };
        use ::Spatial;

        // Make OpenGL resource factory.
        // We'll use this for creating all our vertex buffers, etc.
        let factory = &mut window.factory.clone();

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

        let log = parent_log.new(o!());

        let projection = Arc::new(Mutex::new(get_projection(&window)));
        let first_person = FirstPerson::new(
            [0.5, 0.5, 4.0],
            FirstPersonSettings::keyboard_wasd()
        );
        let first_person_mutex_arc = Arc::new(Mutex::new(first_person));

        let mut render_sys = render::System::new(
            factory,
            render_sys_encoder_channel,
            window.output_color.clone(),
            window.output_stencil.clone(),
            first_person_mutex_arc.clone(),
            projection.clone(),
            &parent_log,
        );

        let (input_sender, input_receiver) = mpsc::channel();
        let control_sys = cell_dweller::ControlSystem::new(
            input_receiver,
            &parent_log,
        );

        // Create SPECS world and, system execution planner
        // for it with two threads.
        //
        // This manages execution of all game systems,
        // i.e. the interaction between sets of components.
        let mut world = specs::World::new();
        world.register::<cell_dweller::CellDweller>();
        world.register::<render::Visual>();
        world.register::<Spatial>();

        // Add some things to the world.

        // Make globe and create a mesh for each of its chunks.
        //
        // TODO: move the geometry generation bits somewhere else;
        // the user shouldn't have to mess with any of this.
        //
        // TODO: don't bake this into the generic app!
        let factory = &mut window.factory.clone();
        let globe = globe::Globe::new_example(&log);
        let globe_view = globe::View::new(&globe, &log);
        let geometry = globe_view.make_geometry(&globe);
        for (vertices, vertex_indices) in geometry {
            let mesh_handle = render_sys.create_mesh(
                factory,
                vertices,
                vertex_indices,
            );
            let mut visual = render::Visual::new();
            // TODO: defer creating the meshes for all these chunks.
            // Best to offload it to a background thread.
            visual.set_mesh_handle(mesh_handle);
            world.create_now()
                .with(visual)
                .with(Spatial::root())
                .build();
        }

        use globe::{ CellPos, Dir };
        let dummy_mesh = render::make_dummy_mesh(
            factory,
            &mut render_sys,
        );
        let mut cell_dweller_visual = render::Visual::new();
        cell_dweller_visual.set_mesh_handle(dummy_mesh);
        world.create_now()
            .with(cell_dweller::CellDweller::new(
                CellPos::default(),
                Dir::default(),
                globe.spec(),
            ))
            .with(cell_dweller_visual)
            .with(Spatial::root())
            .build();

        let mut planner = specs::Planner::new(world, 2);
        planner.add_system(render_sys, "render", 50);
        planner.add_system(control_sys, "control", 100);

        App {
            t: 0.0,
            log: log,
            planner: planner,
            encoder_channel: device_encoder_channel,
            input_sender: input_sender,
            projection: projection,
            first_person: first_person_mutex_arc,
        }
    }

    pub fn run(&mut self, mut window: &mut PistonWindow) {
        use piston::input::*;
        use piston::event_loop::Events;

        info!(self.log, "Starting event loop");

        let mut events = window.events();
        while let Some(e) = events.next(window) {
            self.first_person.lock().unwrap().event(&e);

            if let Some(r) = e.render_args() {
                self.render(&r, &mut window);
            }

            if let Some(_) = e.resize_args() {
                let mut projection = self.projection.lock().unwrap();
                *projection = get_projection(&window);
            }

            if let Some(u) = e.update_args() {
                self.update(&u);
            }

            use piston::input::keyboard::Key;
            use cell_dweller::ControlEvent;
            if let Some(Button::Keyboard(key)) = e.press_args() {
                match key {
                    Key::I => self.input_sender.send(ControlEvent::MoveForward(true)).unwrap(),
                    _ => (),
                }
            }
            if let Some(Button::Keyboard(key)) = e.release_args() {
                match key {
                    Key::I => self.input_sender.send(ControlEvent::MoveForward(false)).unwrap(),
                    _ => (),
                }
            }
        }

        info!(self.log, "Quitting");
    }

    fn render(&mut self, _args: &RenderArgs, window: &mut PistonWindow) {
        // TODO: Systems are currently run on the main thread,
        // so we need to `try_recv` to avoid deadlock.
        // This is only because I don't want to burn CPU, and I've yet
        // to get around to frame/update rate limiting, so I'm
        // relying on Piston's for now.
        use std::sync::mpsc::TryRecvError;
        let mut encoder = match self.encoder_channel.receiver.try_recv() {
            Ok(encoder) => encoder,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => panic!("Render system hung up. That wasn't supposed to happen!"),
        };

        // TODO: what's make_current actually necessary for?
        // Do I even need to do this? (Ripped off `draw_3d`.)
        use piston::window::OpenGLWindow;
        window.window.make_current();

        encoder.flush(&mut window.device);

        self.encoder_channel.sender.send(encoder).unwrap();
    }

    fn update(&mut self, args: &UpdateArgs) {
        self.t += args.dt;
        self.planner.dispatch(args.dt);
    }
}
