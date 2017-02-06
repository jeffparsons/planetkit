use std::sync::{ Arc, Mutex, mpsc };
use piston_window::PistonWindow;
use piston::input::{ UpdateArgs, RenderArgs };
use slog::Logger;
use gfx;
use gfx_device_gl;
use camera_controllers;
use specs;

use render;
use render::{ Visual, Mesh, MeshRepository };
use types::*;
use globe;
use cell_dweller;
use input_adapter::InputAdapter;

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
    input_adapters: Vec<Box<InputAdapter>>,
    // TEMP: Share with rendering system until the rendering system
    // is smart enough to take full ownership of it.
    projection: Arc<Mutex<[[f32; 4]; 4]>>,
    first_person: Arc<Mutex<camera_controllers::FirstPerson>>,
    factory: gfx_device_gl::Factory,
    output_color: gfx::handle::RenderTargetView<gfx_device_gl::Resources, (gfx::format::R8_G8_B8_A8, gfx::format::Srgb)>,
    output_stencil: gfx::handle::DepthStencilView<gfx_device_gl::Resources, (gfx::format::D24_S8, gfx::format::Unorm)>,
    mesh_repo: Arc<Mutex<MeshRepository<gfx_device_gl::Resources>>>,
}

impl App {
    pub fn new(parent_log: &Logger, window: &mut PistonWindow) -> App {
        use camera_controllers::{
            FirstPersonSettings,
            FirstPerson,
        };

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

        let projection = Arc::new(Mutex::new(get_projection(window)));
        let first_person = FirstPerson::new(
            [0.5, 0.5, 4.0],
            FirstPersonSettings::keyboard_wasd()
        );
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
            first_person_mutex_arc.clone(),
            projection.clone(),
            &log,
            mesh_repo_ptr.clone(),
        );

        // Create SPECS world and, system execution planner
        // for it with two threads.
        //
        // This manages execution of all game systems,
        // i.e. the interaction between sets of components.
        let world = specs::World::new();

        let mut planner = specs::Planner::new(world, 2);
        planner.add_system(render_sys, "render", 50);

        App {
            t: 0.0,
            log: log,
            planner: planner,
            encoder_channel: device_encoder_channel,
            input_adapters: Vec::new(),
            projection: projection,
            first_person: first_person_mutex_arc,
            factory: factory.clone(),
            output_color: window.output_color.clone(),
            output_stencil: window.output_stencil.clone(),
            mesh_repo: mesh_repo_ptr,
        }
    }

    // TODO: none of this should be baked into App.
    pub fn temp_remove_me_init(&mut self) {
        use ::Spatial;

        // Add some things to the world.

        // Make globe and create a mesh for each of its chunks.
        //
        // TODO: don't bake this into the generic app!
        let globe = globe::Globe::new_example(&self.log);

        // Find globe surface and put player character on it.
        use globe::{ CellPos, Dir };
        use globe::chunk::Material;
        let mut guy_pos = CellPos::default();
        guy_pos = globe.find_lowest_cell_containing(guy_pos, Material::Air)
            .expect("Uh oh, there's something wrong with our globe.");
        let factory = &mut self.factory.clone();
        let mut mesh_repo = self.mesh_repo.lock().unwrap();
        let axes_mesh = render::make_axes_mesh(
            factory,
            &mut mesh_repo,
        );
        let mut cell_dweller_visual = render::Visual::new_empty();
        cell_dweller_visual.set_mesh_handle(axes_mesh);
        let globe_spec = globe.spec();
        // First add the globe to the world so we can get a
        // handle on its entity.
        let world = self.planner.mut_world();
        let globe_entity = world.create_now()
            .with(globe)
            .build();
        world.create_now()
            .with(cell_dweller::CellDweller::new(
                guy_pos,
                Dir::default(),
                globe_spec,
                Some(globe_entity),
            ))
            .with(cell_dweller_visual)
            .with(Spatial::root())
            .build();
    }

    pub fn run(&mut self, mut window: &mut PistonWindow) {
        use piston::input::*;

        info!(self.log, "Starting event loop");

        let mut events = window.events;
        while let Some(e) = events.next(window) {
            self.first_person.lock().unwrap().event(&e);

            if let Some(r) = e.render_args() {
                self.render(&r, &mut window);
            }

            if e.resize_args().is_some() {
                let mut projection = self.projection.lock().unwrap();
                *projection = get_projection(window);
            }

            if let Some(u) = e.update_args() {
                self.update(&u);
            }

            // Dispatch input events to any systems that care.
            for adapter in &self.input_adapters {
                adapter.handle(&e);
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
        let world = self.planner.mut_world();
        let mut mesh_repo = self.mesh_repo.lock().unwrap();
        let mut visuals = world.write::<Visual>();
        use specs::Join;
        for visual in (&mut visuals).iter() {
            // Even if there's a realized mesh already, the presence of
            // a proto-mesh indicates we need to realize again.
            // (We clear out the proto-mesh when we realize it.)
            let needs_to_be_realized = visual.proto_mesh.is_some();
            if !needs_to_be_realized {
                continue;
            }
            let proto_mesh = visual.proto_mesh.clone().expect("Just ensured this above...");
            let mesh = Mesh::new(
                &mut self.factory,
                proto_mesh.vertexes.clone(),
                proto_mesh.indexes.clone(),
                self.output_color.clone(),
                self.output_stencil.clone(),
            );
            if let Some(existing_mesh_handle) = visual.mesh_handle() {
                // We're replacing an existing mesh that got dirty.
                mesh_repo.replace_mesh(existing_mesh_handle, mesh);
            } else {
                // We're realizing this mesh for the first time.
                let mesh_handle = mesh_repo.add_mesh(mesh);
                visual.set_mesh_handle(mesh_handle);
            }
            visual.proto_mesh = None;
        }
    }

    pub fn add_input_adapter(&mut self, adapter: Box<InputAdapter>) {
        self.input_adapters.push(adapter);
    }
}

impl<'a> App {
    pub fn planner(&'a mut self) -> &'a mut specs::Planner<TimeDelta> {
        &mut self.planner
    }
}
