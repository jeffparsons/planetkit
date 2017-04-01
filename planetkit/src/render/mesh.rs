use gfx;

use super::default_pipeline::pipe;
use super::Vertex;

pub struct Mesh<R: gfx::Resources> {
    data: pipe::Data<R>,
    slice: gfx::Slice<R>,
}

// Allowing sibling modules to reach into semi-private parts
// of the Mesh struct.
pub trait MeshGuts<'a, R: gfx::Resources> {
    fn data(&'a self) -> &'a pipe::Data<R>;
    fn data_mut(&'a mut self) -> &'a mut pipe::Data<R>;
    fn slice(&'a self) -> &'a gfx::Slice<R>;
}

impl<'a, R: gfx::Resources> MeshGuts<'a, R> for Mesh<R> {
    fn data(&'a self) -> &'a pipe::Data<R> {
        &self.data
    }

    fn data_mut(&'a mut self) -> &'a mut pipe::Data<R> {
        &mut self.data
    }

    fn slice(&'a self) -> &'a gfx::Slice<R> {
        &self.slice
    }
}

impl<R: gfx::Resources> Mesh<R> {
    /// Panicks if given an empty vertex or index vector.
    pub fn new<F: gfx::Factory<R>>(
        factory: &mut F,
        vertices: Vec<Vertex>,
        // TODO: accept usize, not u32.
        // That kind of optimisation isn't worthwhile until you hit the video card.
        vertex_indices: Vec<u32>,

        // TODO: this stuff belongs on `render::System` at least by default;
        // we're unlikely to want to customise it per mesh.
        output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
    ) -> Mesh<R> {
        // Don't allow creating empty mesh.
        // Back-end doesn't seem to like this, and it probably represents
        // a mistake if we attempt this anyway.
        assert!(vertices.len() > 0);
        assert!(vertex_indices.len() > 0);

        // Create sampler.
        // TODO: surely there are some sane defaults for this stuff
        // I can just fall back to...
        // TODO: What are these magic numbers? o_0
        use gfx::traits::FactoryExt;
        let texels = [[0x20, 0xA0, 0xC0, 0x00]];
        let (_, texture_view) = factory.create_texture_immutable::<gfx::format::Rgba8>(
            gfx::texture::Kind::D2(1, 1, gfx::texture::AaMode::Single),
            &[&texels]).unwrap();
        let sinfo = gfx::texture::SamplerInfo::new(
            gfx::texture::FilterMethod::Bilinear,
            gfx::texture::WrapMode::Clamp
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
