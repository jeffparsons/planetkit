use na::{ Point2, Point3, Vector2, Vector3 };

use noise;
use icosahedron;

use chunk::{ IntCoord, Chunk, Root, CellPos, Cell };

const ROOT_QUADS: u8 = 10;

// TODO: this belongs at module root
type Vec2 = Vector2<f64>;
type Vec3 = Vector3<f64>;
type Pt2 = Point2<f64>;
type Pt3 = Point3<f64>;

pub struct Spec {
    seed: u32,
    radius: f64,
    // These are the full width/height of a given root quad or chunk's voxmap;
    // i.e. not an exponent.
    root_resolution: IntCoord,
    chunk_resolution: IntCoord,
}

impl Spec {
    pub fn is_valid(&self) -> bool {
        // Chunk resolution needs to be a factor of root resolution.
        let chunks_per_root_side = self.root_resolution / self.chunk_resolution;
        chunks_per_root_side * self.chunk_resolution == self.root_resolution
    }
}

// Project a position in a given root quad into a unit sphere.
// Assumes that one corner is represented in `pt_in_root_quad`
// as (0, 0) and the opposite is (1, 1).
pub fn project(root: Root, pt_in_root_quad: Pt2) -> Pt3 {
    // Each root quad comprises two triangles of the icosahedron.
    // So we need to set up some points and vectors based on the
    // root we're operating in, and then the math depends on which
    // triangle `pt_in_root_quad` is in.
    //
    // See the diagram below for a visual description of how 2-space
    // coordinates on the quad relate to the the icoshaedral vertices.
    // `N` and `S` refer to the north and south triangles respectively.
    //
    //     a    ________  c (0, 1)
    //  (0, 0)  \      /\    N_2
    //    N_0    \ N  /  \   S_1
    //            \  /  S \
    //             \/______\
    //                       d (1, 1)
    //            b (1, 0)     S_0
    //              N_1
    //              S_2
    //
    // TODO: cache all this stuff somewhere. It's tiny, and we'll use it heaps.
    use icosahedron::{ FACES, VERTICES };
    let i_north = root.index as usize * 2;
    let i_south = i_north + 1;
    let north = FACES[i_north];
    let south = FACES[i_south];
    let a: Pt3 = (&VERTICES[north[0]]).into();
    let b: Pt3 = (&VERTICES[north[1]]).into();
    let c: Pt3 = (&VERTICES[north[2]]).into();
    let d: Pt3 = (&VERTICES[south[0]]).into();

    let ab = b - a;
    let ac = c - a;
    let db = b - d;
    let dc = c - d;

    // Decide which triangle we're in.
    if pt_in_root_quad[0] + pt_in_root_quad[1] < 1.0 {
        // In first triangle.
        a + ab * pt_in_root_quad[0] + ac * pt_in_root_quad[1]
    } else {
        // In second triangle.
        d + dc * (1.0 - pt_in_root_quad[0]) + db * (1.0 - pt_in_root_quad[1])
    }
}

// TODO: split out a WorldGen type that handles all the procedural
// generation, because none of that really needs to be tangled
// with the realised Globe.
pub struct Globe {
    spec: Spec,
    // Permutation table for noise
    pt: noise::Seed,
    // TODO: figure out what structure to store these in.
    // You'll never have all chunks loaded in the real world.
    //
    // TODO: you'll probably also want to store some lower-res
    // pseudo-chunks for rendering planets at a distance.
    // But maybe you can put that off?
    chunks: Vec<Chunk>,
}

impl Globe {
    pub fn new(spec: Spec) -> Globe {
        assert!(spec.is_valid(), "Invalid globe spec!");
        let pt = noise::Seed::new(spec.seed);
        let mut globe = Globe {
            spec: spec,
            pt: pt,
            chunks: Vec::new(),
        };
        globe.build_all_chunks();
        globe
    }

    pub fn new_example() -> Globe {
        Globe::new(
            Spec {
                seed: 12,
                radius: 1.0, // TODO: make it ~Earth
                root_resolution: 16,
                chunk_resolution: 4,
            }
        )
    }

    pub fn build_all_chunks(&mut self) {
        let chunks_per_root_side = self.spec.root_resolution / self.spec.chunk_resolution;
        for root_index in 0..ROOT_QUADS {
            let root = Root { index: root_index };
            for y in 0..chunks_per_root_side {
                for x in 0..chunks_per_root_side {
                    let origin = CellPos {
                        root: root.clone(),
                        x: x * self.spec.chunk_resolution,
                        y: y * self.spec.chunk_resolution,
                    };
                    self.build_chunk(origin);
                }
            }
        }
    }

    pub fn build_chunk(&mut self, origin: CellPos) {
        // TODO: get parameters from spec
        let noise = noise::Brownian3::new(noise::perlin3, 4).wavelength(1.0);
        let mut cells: Vec<Cell> = Vec::new();
        let end_x = origin.x + self.spec.chunk_resolution;
        let end_y = origin.y + self.spec.chunk_resolution;
        for cell_y in origin.y..end_y {
            for cell_x in origin.x..end_x {
                // Calculate height for this cell from world spec.

                // TODO: project onto the globe!
                let (x, y, z) = (1.0, 2.0, 3.0);

                // Vary a little bit around 1.0.
                let height = noise.apply(&self.pt, &[x, y, z]) * 0.1 + 1.0;
                cells.push(Cell {
                    height: height,
                });
            }
        }
    }

    // Make vertices and list of indices into that array for triangle faces.
    pub fn make_geometry(&self) -> (Vec<::Vertex>, Vec<u16>) {
        // Output faces for each root quad.
        let mut vertex_data: Vec<::Vertex> = Vec::new();
        let index_vec: Vec<u16> = (0..ROOT_QUADS)
            .flat_map(|root_index| {
                let first_vertex_index = vertex_data.len() as u16;
                // Add vertices of both north and south triangles of quad.
                // Color by root quad.
                let root_color = icosahedron::RAINBOW[root_index as usize];
                let tri_coords: [[f64; 2]; 6] = [
                    // TEMP: match the order of the icosahedron vertices
                    // to minimise differences initially--but this shouldn't
                    // need to be the same at all.
                    // North triangle
                    [0.0, 0.0],
                    [1.0, 0.0],
                    [0.0, 1.0],

                    // South triangle
                    [1.0, 1.0],
                    [0.0, 1.0],
                    [1.0, 0.0],
                ];
                for coord_pair in tri_coords.iter() {
                    let pt3 = project(
                        Root { index: root_index },
                        Pt2::new(coord_pair[0], coord_pair[1]),
                    );
                    let vertex = ::Vertex::new([
                        pt3[0] as f32,
                        pt3[1] as f32,
                        pt3[2] as f32,
                    ], root_color);
                    vertex_data.push(vertex);
                }

                vec![
                    // North face
                    first_vertex_index,
                    first_vertex_index + 1,
                    first_vertex_index + 2,
                    // South face
                    first_vertex_index + 3,
                    first_vertex_index + 4,
                    first_vertex_index + 5,
                ]
            }).collect();

        (vertex_data, index_vec)
    }
}
