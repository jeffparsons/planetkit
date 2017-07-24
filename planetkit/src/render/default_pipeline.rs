use gfx;

use types::Pt3;

// Pretty basic pipeline currently used for terrain.
//
// TODO: determine how aggressively you should be trying
// to re-use this for other things. I.e. what's the cost
// of having lots of pipelines and switching between them.

gfx_vertex_struct!(_Vertex {
    a_pos: [f32; 4] = "a_pos",
    tex_coord: [f32; 2] = "a_tex_coord",
    a_color: [f32; 3] = "a_color",
});

pub type Vertex = _Vertex;

impl Vertex {
    pub fn new(pos: [f32; 3], color: [f32; 3]) -> Vertex {
        Vertex {
            a_pos: [pos[0], pos[1], pos[2], 1.0],
            a_color: color,
            tex_coord: [0.0, 0.0],
        }
    }

    pub fn new_from_pt3(pos: Pt3, color: [f32; 3]) -> Vertex {
        Vertex::new([pos[0] as f32, pos[1] as f32, pos[2] as f32], color)
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
