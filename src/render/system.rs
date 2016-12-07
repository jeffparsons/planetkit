use std::sync::mpsc;
use gfx;
use gfx::Primitive;
use gfx::state::Rasterizer;

// System to render all visible entities. This is back-end agnostic;
// i.e. nothing in it should be tied to OpenGL, Vulkan, etc.

gfx_vertex_struct!(
    _Vertex {
        a_pos: [f32; 4] = "a_pos",
        tex_coord: [f32; 2] = "a_tex_coord",
        a_color: [f32; 3] = "a_color",
    }
);

pub type Vertex = _Vertex;

impl Vertex {
    pub fn new(pos: [f32; 3], color: [f32; 3]) -> Vertex {
        Vertex {
            a_pos: [pos[0], pos[1], pos[2], 1.0],
            a_color: color,
            tex_coord: [0.0, 0.0],
        }
    }
}

gfx_pipeline!(
    pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        u_model_view_proj: gfx::Global<[[f32; 4]; 4]> = "u_model_view_proj",
        t_color: gfx::TextureSampler<[f32; 4]> = "t_color",
        out_color: gfx::RenderTarget<gfx::format::Srgba8> = "o_color",
        out_depth: gfx::DepthTarget<gfx::format::DepthStencil> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
);

pub struct Mesh<R: gfx::Resources> {
    data: pipe::Data<R>,
    slice: gfx::Slice<R>,
}

impl<R: gfx::Resources> Mesh<R> {
    pub fn new<F: gfx::Factory<R>>(
        factory: &mut F,
        vertices: Vec<Vertex>,
        vertex_indices: Vec<u32>,

        // TODO: this stuff belongs on `Draw` at least by default;
        // we're unlikely to want to customise it per mesh.
        output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
    ) -> Mesh<R> {
        // Create sampler.
        // TODO: surely there are some sane defaults for this stuff
        // I can just fall back to...
        // TODO: What are these magic numbers? o_0
        use gfx::traits::FactoryExt;
        let texels = [[0x20, 0xA0, 0xC0, 0x00]];
        let (_, texture_view) = factory.create_texture_const::<gfx::format::Rgba8>(
            gfx::tex::Kind::D2(1, 1, gfx::tex::AaMode::Single),
            &[&texels]).unwrap();
        let sinfo = gfx::tex::SamplerInfo::new(
            gfx::tex::FilterMethod::Bilinear,
            gfx::tex::WrapMode::Clamp
        );

        let index_data: &[u32] = vertex_indices.as_slice();
        let (vbuf, slice) = factory.create_vertex_buffer_with_slice(
            &vertices, index_data
        );
        let data = pipe::Data {
            vbuf: vbuf.clone(),
            u_model_view_proj: [[0.0; 4]; 4],
            t_color: (texture_view, factory.create_sampler(sinfo)),
            out_color: output_color,
            out_depth: output_stencil,
        };
        Mesh {
            data: data,
            slice: slice,
        }
    }
}

// Bi-directional channel to send Encoder between game thread(s)
// (as managed by Specs), and the thread owning the graphics device.
pub struct EncoderChannel<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
    pub receiver: mpsc::Receiver<gfx::Encoder<R, C>>,
    pub sender: mpsc::Sender<gfx::Encoder<R, C>>,
}

pub struct Draw<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
    // TODO: multiple PSOs
    pso: gfx::PipelineState<R, pipe::Meta>,
    meshes: Vec<Mesh<R>>,
    encoder_channel: EncoderChannel<R, C>,
    output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
    output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
}

impl<R: gfx::Resources, C: gfx::CommandBuffer<R>> Draw<R, C> {
    pub fn new<F: gfx::Factory<R>>(
        factory: &mut F,
        encoder_channel: EncoderChannel<R, C>,
        output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
    ) -> Draw<R, C> {
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

        Draw {
            pso: pso,
            meshes: Vec::new(),
            encoder_channel: encoder_channel,
            output_color: output_color,
            output_stencil: output_stencil,
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
            mesh.data.u_model_view_proj = model_view_projection;
            encoder.draw(
                &mesh.slice,
                &self.pso,
                &mesh.data,
            );
        }

        self.encoder_channel.sender.send(encoder).unwrap();
    }
}