use gfx;
use gfx::Primitive;
use gfx::state::Rasterizer;

// Most of this boilerplate stuff adapted from
// <https://github.com/PistonDevelopers/piston-examples/blob/master/src/cube.rs>.
// TODO: This is incomprehensible! Do something about it.

// Most of this was then brought into this `draw` module,
// which is currently super-specific to `globe`, but will soon
// be abstracted to handle different kinds of meshes.

// SHOULD BE BACK-END AGNOSTIC -- I.e. try not to tie this to OpenGL.

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

struct Mesh<R: gfx::Resources> {
    data: pipe::Data<R>,
    slice: gfx::Slice<R>,
}

impl<R: gfx::Resources> Mesh<R> {
    fn new(
        data: pipe::Data<R>,
        slice: gfx::Slice<R>,
    ) -> Mesh<R> {
        Mesh {
            data: data,
            slice: slice,
        }
    }
}

pub struct Draw<R: gfx::Resources> {
    // TODO: multiple PSOs
    pso: gfx::PipelineState<R, pipe::Meta>,
    meshes: Vec<Mesh<R>>,
}

impl<R: gfx::Resources> Draw<R> {
    pub fn new<F: gfx::Factory<R>>(
        factory: &mut F,
        out_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        out_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
        vertices: Vec<Vertex>,
        vertex_indices: Vec<u32>,
    ) -> Draw<R> {
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

        // Create sampler.
        // TODO: surely there are some sane defaults for this stuff
        // I can just fall back to...
        // TODO: What are these magic numbers? o_0
        let texels = [[0x20, 0xA0, 0xC0, 0x00]];
        let (_, texture_view) = factory.create_texture_const::<gfx::format::Rgba8>(
            gfx::tex::Kind::D2(1, 1, gfx::tex::AaMode::Single),
            &[&texels]).unwrap();
        let sinfo = gfx::tex::SamplerInfo::new(
            gfx::tex::FilterMethod::Bilinear,
            gfx::tex::WrapMode::Clamp
        );

        // Create bundle to render per mesh, minus PSO.
        let index_data: &[u32] = vertex_indices.as_slice();
        let (vbuf, slice) = factory.create_vertex_buffer_with_slice(
            &vertices, index_data
        );
        let data = pipe::Data {
            vbuf: vbuf.clone(),
            u_model_view_proj: [[0.0; 4]; 4],
            t_color: (texture_view, factory.create_sampler(sinfo)),
            out_color: out_color,
            out_depth: out_stencil,
        };

        let meshes = vec![
            Mesh::new(data, slice),
        ];
        Draw {
            pso: pso,
            meshes: meshes,
        }
    }

    pub fn draw<C: gfx::CommandBuffer<R>>(
        &mut self,
        encoder: &mut gfx::Encoder<R, C>,
        model_view_projection: [[f32; 4]; 4],
    ) {
        for mesh in &mut self.meshes {
            mesh.data.u_model_view_proj = model_view_projection;
            encoder.draw(
                &mesh.slice,
                &self.pso,
                &mesh.data,
            );
        }
    }
}
