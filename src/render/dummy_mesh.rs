use gfx;

use super::{ System, MeshHandle, Vertex };
use ::globe::icosahedron;

const BRIGHT_RED: [f32; 3] = [ 1.0, 0.0, 0.0 ];

pub fn make_dummy_mesh<
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    F: gfx::Factory<R>,
>(
    factory: &mut F,
    render_system: &mut System<R, C>,
) -> MeshHandle {
    let vertex_data: Vec<Vertex> = icosahedron::VERTICES
        .iter()
        .map(|v| {
            Vertex::new([
                // TODO: this multiplier is only because the globe I'm testing
                // with is currently stupidly small.
                v[0] as f32 * 0.01,
                v[1] as f32 * 0.01,
                v[2] as f32 * 0.01,
            ], BRIGHT_RED)
        })
        .collect();
    let index_vec: Vec<u32> = icosahedron::FACES
        .iter()
        .flat_map(|f| vec![f[0], f[1], f[2]])
        .map(|vi| vi as u32)
        .collect();

    // Register mesh with render system and return it.
    render_system.create_mesh(factory, vertex_data, index_vec)
}