use gfx;

use super::{ System, MeshHandle, Vertex };
use ::globe::icosahedron;

pub const RAINBOW: [[f32; 3]; 10] = [
    [ 1.0, 0.0, 0.0 ],
    [ 1.0, 0.5, 0.0 ],
    [ 1.0, 1.0, 0.0 ],
    [ 0.5, 1.0, 0.0 ],
    [ 0.0, 1.0, 0.0 ],
    [ 0.0, 1.0, 0.5 ],
    [ 0.0, 1.0, 1.0 ],
    [ 0.0, 0.5, 1.0 ],
    [ 0.0, 0.0, 1.0 ],
    [ 0.5, 0.0, 1.0 ],
];

pub fn make_dummy_mesh<
    R: gfx::Resources,
    C: gfx::CommandBuffer<R>,
    F: gfx::Factory<R>,
>(
    factory: &mut F,
    render_system: &mut System<R, C>,
) -> MeshHandle {
    let mut vertex_data = Vec::<Vertex>::new();
    let mut index_vec = Vec::<u32>::new();
    let mut color_index = 0.0;
    for color in RAINBOW.iter() {
        let first_vertex_index = vertex_data.len() as u32;
        for vertex in icosahedron::VERTICES.iter() {
            vertex_data.push(
                Vertex::new([
                    // TODO: this multiplier is only because the globe I'm testing
                    // with is currently stupidly small.
                    vertex[0] as f32 * 0.01,
                    vertex[1] as f32 * 0.01,
                    // Space the blobs out a bit.
                    vertex[2] as f32 * 0.01 + color_index * 0.03,
                ], *color)
            );
        }
        for face in icosahedron::FACES.iter() {
            index_vec.push(first_vertex_index + face[0] as u32);
            index_vec.push(first_vertex_index + face[1] as u32);
            index_vec.push(first_vertex_index + face[2] as u32);
        }
        color_index += 1.0;
    }

    // Register mesh with render system and return it.
    render_system.create_mesh(factory, vertex_data, index_vec)
}