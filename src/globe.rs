use noise;
use icosahedron;

pub struct Spec {
    seed: u32,
    radius: f64,
    // These are the full width/height of a given root quad or chunk's voxmap;
    // i.e. not an exponent.
    root_resolution: u64,
    chunk_resolution: u64,
}

impl Spec {
    pub fn is_valid(&self) -> bool {
        // Chunk resolution needs to be a factor of root resolution.
        let chunks_per_root_side = self.root_resolution / self.chunk_resolution;
        chunks_per_root_side * self.chunk_resolution == self.root_resolution
    }
}

pub struct Globe {
    spec: Spec,
    // Permutation table for noise
    pt: noise::Seed,
}

impl Globe {
    pub fn new(spec: Spec) -> Globe {
        assert!(spec.is_valid(), "Invalid globe spec!");
        let pt = noise::Seed::new(spec.seed);
        Globe {
            spec: spec,
            pt: pt,
        }
    }

    pub fn new_example() -> Globe {
        Globe::new(
            Spec {
                seed: 12,
                radius: 1.2, // TODO: make it ~Earth
                root_resolution: 256,
                chunk_resolution: 16,
            }
        )
    }

    // Make vertices and list of indices into that array for triangle faces.
    pub fn make_geometry(&self) -> (Vec<::Vertex>, Vec<u16>) {
        // Massage vertex data into form that gfx wants.
        // Set up some noise so we can mutate the base
        // icosahedron vertices.
        let noise = noise::Brownian3::new(noise::perlin3, 4).wavelength(1.0);
        let mutated_vertices: Vec<::Vertex> = icosahedron::VERTICES
            .iter()
            .map(|v| {
                let (x, y, z) = (
                    v[0] * self.spec.radius,
                    v[1] * self.spec.radius,
                    v[2] * self.spec.radius,
                );
                // Vary a little bit around 1.0.
                let val = noise.apply(&self.pt, &[x, y, z]) * 0.1 + 1.0;
                // Set the color later when we clone it.
                // TODO: This code is a mess. Hack first, clean it up later.
                ::Vertex::new([
                    (x * val) as f32,
                    (y * val) as f32,
                    (z * val) as f32,
                ], [0.0, 0.0, 0.0])
            })
            .collect();

        // Give each face its own unique copies of the vertices,
        // so that we can colour the faces independently.
        let mut vertex_data: Vec<::Vertex> = Vec::new();
        let mut face_index: usize = 0;
        let index_vec: Vec<u16> = icosahedron::FACES
            .iter()
            .flat_map(|f| {
                let first_vertex_index = vertex_data.len();
                for v in 0..3 {
                    let mut colored_vertex = mutated_vertices[f[v]].clone();
                    // Give adjacent triangles the same colour.
                    // We want to highlight the connection between
                    // the two triangles that make up each quad.
                    colored_vertex.a_color = icosahedron::RAINBOW[face_index / 2];
                    vertex_data.push(colored_vertex);
                }
                face_index += 1;
                vec![
                    first_vertex_index,
                    first_vertex_index + 1,
                    first_vertex_index + 2,
                ]
            })
            .map(|vi| vi as u16)
            .collect();

        (vertex_data, index_vec)
    }
}
