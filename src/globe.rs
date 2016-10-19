use rand;
use rand::Rng;

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
        // Chunk resolution needs to divide perfectly into root resolution.
        let calculated_root_resolution = self.chunks_per_root_side() * self.chunk_resolution;
        calculated_root_resolution == self.root_resolution
    }

    pub fn chunks_per_root_side(&self) -> IntCoord {
        // Assume chunk resolution divides perfectly into root resolution.
        self.root_resolution / self.chunk_resolution
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
    let pos_on_icosahedron = if pt_in_root_quad[0] + pt_in_root_quad[1] < 1.0 {
        // In first triangle.
        a + ab * pt_in_root_quad[0] + ac * pt_in_root_quad[1]
    } else {
        // In second triangle.
        d + dc * (1.0 - pt_in_root_quad[0]) + db * (1.0 - pt_in_root_quad[1])
    };
    use na::Norm;
    *pos_on_icosahedron.as_vector().normalize().as_point()
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
        let mut vertex_data: Vec<::Vertex> = Vec::new();
        let mut index_data: Vec<u16> = Vec::new();

        // Build geometry for each chunk into our buffers.
        let chunks_per_root_side = self.spec.chunks_per_root_side();
        for root_index in 0..ROOT_QUADS {
            for y in 0..chunks_per_root_side {
                for x in 0..chunks_per_root_side {
                    let origin = CellPos {
                        root: root_index.into(),
                        x: x * self.spec.chunk_resolution,
                        y: y * self.spec.chunk_resolution,
                    };
                    self.make_chunk_geometry(
                        origin,
                        &mut vertex_data,
                        &mut index_data
                    );
                }
            }
        }

        (vertex_data, index_data)
    }

    pub fn make_chunk_geometry(
        &self,
        origin: CellPos,
        vertex_data: &mut Vec<::Vertex>,
        index_data: &mut Vec<u16>
    ) {
        let end_x = origin.x + self.spec.chunk_resolution;
        let end_y = origin.y + self.spec.chunk_resolution;
        for cell_y in origin.y..end_y {
            for cell_x in origin.x..end_x {
                // TEMP: Randomly mutate cell color to make it easier to see edges.
                let root_color = icosahedron::RAINBOW[origin.root.index as usize];
                let mut cell_color = root_color;
                let mut rng = rand::thread_rng();
                for mut color_channel in cell_color.iter_mut() {
                    *color_channel *= rng.next_f32();
                }

                // TODO: use functions that return just the bit they care
                // about and... maths. This is silly.
                let first_vertex_index = vertex_data.len() as u16;

                // Output a quad for this cell starting from its
                // center point and going to the next on (x, y).
                // Name anti-clockwise starting from (0, 0).
                // TODO: output hexagons+pentagons instead.
                let a = CellPos {
                    x: cell_x,
                    y: cell_y,
                    root: origin.root,
                };
                let mut b = a.clone();
                b.x += 1;
                let mut c = b.clone();
                c.y += 1;
                let mut d = c.clone();
                d.x -= 1;

                for cell_pos in [a, b, c, d].iter() {
                    // TODO: use actual chunk data to render this stuff.
                    let pt3 = self.cell_center(*cell_pos);
                    vertex_data.push(::Vertex::new([
                        pt3[0] as f32,
                        pt3[1] as f32,
                        pt3[2] as f32,
                    ], cell_color));
                }

                // Output two faces for the cell. For lack of
                // a better idea, just going with north and south;
                // kinda like the inverse of when we were joining
                // icosahedral triangles into quads.
                index_data.extend(&[
                    // North face
                    first_vertex_index,
                    first_vertex_index + 1,
                    first_vertex_index + 3,
                    // South face
                    first_vertex_index + 2,
                    first_vertex_index + 3,
                    first_vertex_index + 1,
                ]);
            }
        }
    }

    fn cell_center(&self, cell_pos: CellPos) -> Pt3 {
        let res = self.spec.root_resolution as f64;
        let pt_in_root_quad = Pt2::new(
            cell_pos.x as f64 / res,
            cell_pos.y as f64 / res,
        );
        project(cell_pos.root, pt_in_root_quad)
    }
}
