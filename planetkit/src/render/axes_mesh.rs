use super::{ProtoMesh, Vertex};
use crate::globe::icosahedron;

pub const GRAY: [f32; 3] = [0.5, 0.5, 0.5];
pub const RED: [f32; 3] = [1.0, 0.0, 0.0];
pub const GREEN: [f32; 3] = [0.0, 1.0, 0.0];
pub const BLUE: [f32; 3] = [0.0, 0.0, 1.0];

pub fn make_axes_mesh() -> ProtoMesh {
    let mut vertex_data = Vec::<Vertex>::new();
    let mut index_vec = Vec::<u32>::new();

    let spacing = 0.5;
    let x_spacing = [spacing, 0.0, 0.0];
    let y_spacing = [0.0, spacing, 0.0];
    let z_spacing = [0.0, 0.0, spacing];

    add_blob(&mut vertex_data, &mut index_vec, GRAY, [0.0, 0.0, 0.0]);
    add_axis(&mut vertex_data, &mut index_vec, RED, x_spacing);
    add_axis(&mut vertex_data, &mut index_vec, GREEN, y_spacing);
    add_axis(&mut vertex_data, &mut index_vec, BLUE, z_spacing);

    ProtoMesh::new(vertex_data, index_vec)
}

fn add_axis(
    vertex_data: &mut Vec<Vertex>,
    index_vec: &mut Vec<u32>,
    color: [f32; 3],
    spacing: [f32; 3],
) {
    for i in 0..3 {
        // Space the blobs out a bit.
        let offset = [
            (i as f32 + 1.0) * spacing[0],
            (i as f32 + 1.0) * spacing[1],
            (i as f32 + 1.0) * spacing[2],
        ];
        add_blob(vertex_data, index_vec, color, offset);
    }
}

fn add_blob(
    vertex_data: &mut Vec<Vertex>,
    index_vec: &mut Vec<u32>,
    color: [f32; 3],
    offset: [f32; 3],
) {
    let first_vertex_index = vertex_data.len() as u32;
    for vertex in &icosahedron::VERTICES {
        vertex_data.push(Vertex::new(
            [
                vertex[0] as f32 * 0.2 + offset[0],
                vertex[1] as f32 * 0.2 + offset[1],
                vertex[2] as f32 * 0.2 + offset[2],
            ],
            color,
        ));
    }
    for face in &icosahedron::FACES {
        index_vec.push(first_vertex_index + face[0] as u32);
        index_vec.push(first_vertex_index + face[1] as u32);
        index_vec.push(first_vertex_index + face[2] as u32);
    }
}
