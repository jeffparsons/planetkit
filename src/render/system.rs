use std::sync::{ Arc, Mutex };
use gfx;
use gfx::Primitive;
use gfx::state::Rasterizer;
use vecmath;
use camera_controllers;
use specs;
use slog::Logger;

use super::default_pipeline::pipe;
use super::mesh::{ Mesh, MeshGuts };
use super::EncoderChannel;
use ::types::*;

// System to render all visible entities. This is back-end agnostic;
// i.e. nothing in it should be tied to OpenGL, Vulkan, etc.

pub struct System<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
    log: Logger,
    // TODO: multiple PSOs
    pso: gfx::PipelineState<R, pipe::Meta>,
    meshes: Vec<Mesh<R>>,
    encoder_channel: EncoderChannel<R, C>,
    output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
    output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
    first_person: Arc<Mutex<camera_controllers::FirstPerson>>,
    projection: Arc<Mutex<[[f32; 4]; 4]>>,
}

impl<R: gfx::Resources, C: gfx::CommandBuffer<R>> System<R, C> {
    pub fn new<F: gfx::Factory<R>>(
        factory: &mut F,
        encoder_channel: EncoderChannel<R, C>,
        output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
        first_person: Arc<Mutex<camera_controllers::FirstPerson>>,
        projection: Arc<Mutex<[[f32; 4]; 4]>>,
        parent_log: &Logger,
    ) -> System<R, C> {
        // Create pipeline state object.
        use gfx::traits::FactoryExt;
        let vs_bytes = include_bytes!("../shaders/copypasta_150.glslv");
        let ps_bytes = include_bytes!("../shaders/copypasta_150.glslf");
        let program = factory.link_program(vs_bytes, ps_bytes).unwrap();
        let pso = factory.create_pipeline_from_program(
            &program,
            Primitive::TriangleList,
            Rasterizer::new_fill().with_cull_back(),
            pipe::new()
        ).unwrap();

        System {
            pso: pso,
            meshes: Vec::new(),
            encoder_channel: encoder_channel,
            output_color: output_color,
            output_stencil: output_stencil,
            first_person: first_person,
            projection: projection,
            log: parent_log.new(o!("system" => "render")),
        }
    }

    pub fn add_mesh(&mut self, mesh: Mesh<R>) {
        debug!(self.log, "Adding mesh...");

        self.meshes.push(mesh);
    }

    pub fn draw(
        &mut self,
        dt: TimeDelta,
    ) {
        // TODO: Systems are currently run on the main thread,
        // so we need to `try_recv` to avoid deadlock.
        // This is only because I don't want to burn CPU, and I've yet
        // to get around to frame/update rate limiting, so I'm
        // relying on Piston's for now.
        use std::sync::mpsc::TryRecvError;
        let mut encoder = match self.encoder_channel.receiver.try_recv() {
            Ok(encoder) => encoder,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => panic!("Device owner hung up. That wasn't supposed to happen!"),
        };

        const CLEAR_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];
        encoder.clear(&self.output_color, CLEAR_COLOR);
        encoder.clear_depth(&self.output_stencil, 1.0);

        let fp = self.first_person.lock().unwrap();
        let projection = self.projection.lock().unwrap();
        // TODO: store this
        let model: vecmath::Matrix4<f32> = vecmath::mat4_id();
        let model_view_projection = camera_controllers::model_view_projection(
            model,
            fp.camera(dt).orthogonal(),
            *projection
        );
        for mesh in &mut self.meshes {
            mesh.data_mut().u_model_view_proj = model_view_projection;
            encoder.draw(
                mesh.slice(),
                &self.pso,
                mesh.data(),
            );
        }

        self.encoder_channel.sender.send(encoder).unwrap();
    }
}

impl<R, C> specs::System<TimeDelta> for System<R, C> where
R: 'static + gfx::Resources,
C: 'static + gfx::CommandBuffer<R> + Send,
{
    fn run(&mut self, arg: specs::RunArg, dt: TimeDelta) {
        // Stop Specs from freaking out about us not having fetched
        // components and assuming we've panicked.
        arg.fetch(|_w| { () });

        // TODO: implement own "extrapolated time" concept or similar
        // to decide how often we should actually be trying to render?
        // See https://github.com/PistonDevelopers/piston/issues/193

        self.draw(dt);
    }
}
