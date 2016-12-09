use std::sync::{ Arc, Mutex };
use gfx;
use gfx::Primitive;
use gfx::state::Rasterizer;
use camera_controllers;

use super::default_pipeline::pipe;
use super::mesh::{ Mesh, MeshGuts };
use super::EncoderChannel;

// System to render all visible entities. This is back-end agnostic;
// i.e. nothing in it should be tied to OpenGL, Vulkan, etc.

pub struct System<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
    // TODO: multiple PSOs
    pso: gfx::PipelineState<R, pipe::Meta>,
    meshes: Vec<Mesh<R>>,
    encoder_channel: EncoderChannel<R, C>,
    output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
    output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
    first_person: Arc<Mutex<camera_controllers::FirstPerson>>,
}

impl<R: gfx::Resources, C: gfx::CommandBuffer<R>> System<R, C> {
    pub fn new<F: gfx::Factory<R>>(
        factory: &mut F,
        encoder_channel: EncoderChannel<R, C>,
        output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
        first_person: Arc<Mutex<camera_controllers::FirstPerson>>,
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
        }
    }

    pub fn add_mesh(&mut self, mesh: Mesh<R>) {
        self.meshes.push(mesh);
    }

    pub fn draw(
        &mut self,
        model_view_projection: [[f32; 4]; 4],
    ) {
        let mut encoder = self.encoder_channel.receiver.recv().expect("Device owner hung up. That wasn't supposed to happen!");

        const CLEAR_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];
        encoder.clear(&self.output_color, CLEAR_COLOR);
        encoder.clear_depth(&self.output_stencil, 1.0);

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
