use camera_controllers;
use gfx;
use gfx_device_gl;
use piston::input::{RenderArgs, UpdateArgs};
use piston_window::PistonWindow;
use slog::Logger;
use specs;
use std::sync::{mpsc, Arc, Mutex};

use crate::input_adapter::InputAdapter;
use crate::render;
use crate::render::{Mesh, MeshRepository, Visual};
use crate::types::*;

fn get_projection(w: &PistonWindow) -> [[f32; 4]; 4] {
    use camera_controllers::CameraPerspective;
    use piston::window::Window;

    let draw_size = w.window.draw_size();
    CameraPerspective {
        fov: 90.0,
        near_clip: 0.01,
        far_clip: 100.0,
        aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32),
    }
    .projection()
}

pub struct App {
    t: TimeDelta,
    log: Logger,
    world: specs::World,
    dispatcher: specs::Dispatcher<'static, 'static>,
    encoder_channel: render::EncoderChannel<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    input_adapters: Vec<Box<dyn InputAdapter>>,
    // TEMP: Share with rendering system until the rendering system
    // is smart enough to take full ownership of it.
    projection: Arc<Mutex<[[f32; 4]; 4]>>,
    first_person: Arc<Mutex<camera_controllers::FirstPerson>>,
    factory: gfx_device_gl::Factory,
    output_color: gfx::handle::RenderTargetView<
        gfx_device_gl::Resources,
        (gfx::format::R8_G8_B8_A8, gfx::format::Srgb),
    >,
    output_stencil: gfx::handle::DepthStencilView<
        gfx_device_gl::Resources,
        (gfx::format::D24_S8, gfx::format::Unorm),
    >,
    mesh_repo: Arc<Mutex<MeshRepository<gfx_device_gl::Resources>>>,
    window: PistonWindow,
}

impl App {
    // Add all your systems before passing the dispatcher in.
    pub fn new(
        parent_log: &Logger,
        mut window: PistonWindow,
        mut world: specs::World,
        dispatcher_builder: specs::DispatcherBuilder<'static, 'static>,
    ) -> App {
        use camera_controllers::{FirstPerson, FirstPersonSettings};

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
        let enc1 = window.factory.create_command_buffer().into();
        let enc2 = window.factory.create_command_buffer().into();
        // TODO: this carefully sending one encoder to each
        // channel is only because I'm temporarily calling
        // the rendering system synchronously until I get
        // around to turning it into a Specs system. Juggling like
        // this prevents deadlock.
        render_sys_encoder_channel.sender.send(enc1).unwrap();
        device_encoder_channel.sender.send(enc2).unwrap();

        let log = parent_log.new(o!());

        let projection = Arc::new(Mutex::new(get_projection(&window)));
        let first_person = FirstPerson::new([0.5, 0.5, 4.0], FirstPersonSettings::keyboard_wasd());
        let first_person_mutex_arc = Arc::new(Mutex::new(first_person));

        let mesh_repo = MeshRepository::new(
            window.output_color.clone(),
            window.output_stencil.clone(),
            &log,
        );

        let factory = &mut window.factory.clone();
        let mesh_repo_ptr = Arc::new(Mutex::new(mesh_repo));
        let render_sys = render::System::new(
            factory,
            render_sys_encoder_channel,
            window.output_color.clone(),
            window.output_stencil.clone(),
            projection.clone(),
            &log,
            mesh_repo_ptr.clone(),
        );

        let dispatcher_builder = dispatcher_builder
            // Wait for unknown systems to finish before rendering.
            .with_barrier()
            .with(render_sys, "render", &[]);

        let mut dispatcher = dispatcher_builder.build();
        // We'll be wanting to poke things into queues before we first
        // call `dispatch`, so ensure all resources exist.
        dispatcher.setup(&mut world.res);

        App {
            t: 0.0,
            log,
            world,
            dispatcher,
            encoder_channel: device_encoder_channel,
            input_adapters: Vec::new(),
            projection,
            first_person: first_person_mutex_arc,
            factory: factory.clone(),
            output_color: window.output_color.clone(),
            output_stencil: window.output_stencil.clone(),
            mesh_repo: mesh_repo_ptr,
            window,
        }
    }

    pub fn run(&mut self) {
        use piston::input::*;

        info!(self.log, "Starting event loop");

        let mut events = self.window.events;
        while let Some(e) = events.next(&mut self.window) {
            self.first_person.lock().unwrap().event(&e);

            if let Some(r) = e.render_args() {
                self.render(&r);
            }

            if e.resize_args().is_some() {
                let mut projection = self.projection.lock().unwrap();
                *projection = get_projection(&self.window);
            }

            if let Some(u) = e.update_args() {
                self.update(u);
            }

            // Dispatch input events to any systems that care.
            if let Event::Input(input) = e {
                for adapter in &self.input_adapters {
                    adapter.handle(&input);
                }
            }
        }

        info!(self.log, "Quitting");
    }

    fn render(&mut self, _args: &RenderArgs) {
        // TODO: Systems are currently run on the main thread,
        // so we need to `try_recv` to avoid deadlock.
        // This is only because I don't want to burn CPU, and I've yet
        // to get around to frame/update rate limiting, so I'm
        // relying on Piston's for now.
        use std::sync::mpsc::TryRecvError;
        let mut encoder = match self.encoder_channel.receiver.try_recv() {
            Ok(encoder) => encoder,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                panic!("Render system hung up. That wasn't supposed to happen!")
            }
        };

        // TODO: what's make_current actually necessary for?
        // Do I even need to do this? (Ripped off `draw_3d`.)
        use piston::window::OpenGLWindow;
        self.window.window.make_current();

        encoder.flush(&mut self.window.device);

        self.encoder_channel.sender.send(encoder).unwrap();
    }

    fn update(&mut self, args: UpdateArgs) {
        self.t += args.dt;

        self.world.write_resource::<TimeDeltaResource>().0 = args.dt;
        self.dispatcher.dispatch(&self.world.res);
        self.world.maintain();

        self.realize_proto_meshes();
    }

    // This whole thing is a horrible hack around
    // not being able to create GL resource factories
    // on other threads. It's acting as a proof that
    // I can make this work, at which point I should gut
    // the whole disgusting thing and find a better way
    // to work around the root problem.
    fn realize_proto_meshes(&mut self) {
        // NOTE: it is essential that we lock the world first.
        // Otherwise we could dead-lock against, e.g., the render
        // system while it's trying to lock the mesh repository.
        let mut mesh_repo = self.mesh_repo.lock().unwrap();
        let mut visuals = self.world.write_storage::<Visual>();
        use specs::Join;
        for visual in (&mut visuals).join() {
            // Even if there's a realized mesh already, the presence of
            // a proto-mesh indicates we need to realize again.
            // (We clear out the proto-mesh when we realize it.)
            let needs_to_be_realized = visual.proto_mesh.is_some();
            if !needs_to_be_realized {
                continue;
            }
            let proto_mesh = visual
                .proto_mesh
                .clone()
                .expect("Just ensured this above...");
            // Realize the mesh and hand it off to the mesh repository.
            let mesh = Mesh::new(
                &mut self.factory,
                proto_mesh.vertexes.clone(),
                proto_mesh.indexes.clone(),
                self.output_color.clone(),
                self.output_stencil.clone(),
            );
            let mesh_pointer = mesh_repo.add_mesh(mesh);
            // We may or may not be replacing a pointer to another mesh here;
            // if we are, then the old mesh (assuming it isn't being used for anything else)
            // will be discarded.
            visual.set_mesh_pointer(mesh_pointer);
            visual.proto_mesh = None;
        }
        // REVISIT: I'm guessing the underlying `Storage::sync_pending` API will change in future;
        // keep an eye on it.
        mesh_repo.collect_garbage();
    }

    pub fn add_input_adapter(&mut self, adapter: Box<dyn InputAdapter>) {
        self.input_adapters.push(adapter);
    }
}

impl<'a> App {
    pub fn world_mut(&'a mut self) -> &'a mut specs::World {
        &mut self.world
    }

    // Hacks to get around borrowing App twice mutably.
    pub fn world_and_window_mut(&'a mut self) -> (&'a mut specs::World, &'a mut PistonWindow) {
        (&mut self.world, &mut self.window)
    }
}
