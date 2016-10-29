use rand;
use rand::Rng;

use noise;

use types::*;
use super::icosahedron;
use super::Root;
use super::chunk::{ Chunk, CellPos, Cell };
use super::spec::Spec;

const ROOT_QUADS: u8 = 10;

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
                seed: 13,
                radius: 1.0, // TODO: make it ~Earth
                root_resolution: 64,
                chunk_resolution: 16,
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
                        root: root,
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
        let noise = noise::Brownian3::new(noise::open_simplex3::<f64>, 6).wavelength(1.0);
        let mut cells: Vec<Cell> = Vec::new();
        let end_x = origin.x + self.spec.chunk_resolution;
        let end_y = origin.y + self.spec.chunk_resolution;
        for cell_y in origin.y..end_y {
            for cell_x in origin.x..end_x {
                // Calculate height for this cell from world spec.
                // To do this, project the cell onto a unit sphere
                // and sample 3D simplex noise to get a height value.
                //
                // TODO: split out a proper world generator
                // that layers in lots of different kinds of noise etc.
                let cell_pos = CellPos {
                    root: origin.root,
                    x: cell_x,
                    y: cell_y,
                    // TODO: maybe don't use cell_center;
                    // i.e. use a special method that explicitly
                    // ignores the z-coordinate for use in
                    // operations like noise sampling.
                    // z: 1,
                };
                let pt3 = self.cell_center(cell_pos);

                // Vary a little bit around 1.0.
                let delta =
                    noise.apply(&self.pt, pt3.as_ref())
                    * self.spec.radius
                    * 0.2;
                let height = self.spec.radius + delta;
                cells.push(Cell {
                    height: height,
                });
            }
        }
        self.chunks.push(Chunk {
            origin: origin,
            cells: cells,
        });
    }

    // Make vertices and list of indices into that array for triangle faces.
    pub fn make_geometry(&self) -> (Vec<::Vertex>, Vec<u32>) {
        let mut vertex_data: Vec<::Vertex> = Vec::new();
        let mut index_data: Vec<u32> = Vec::new();

        // Build geometry for each chunk into our buffers.
        for chunk in &self.chunks {
            // TODO: factor out
            self.make_chunk_geometry(
                &chunk,
                &mut vertex_data,
                &mut index_data,
            );
        }
        (vertex_data, index_data)
    }

    pub fn make_chunk_geometry(
        &self,
        chunk: &Chunk,
        vertex_data: &mut Vec<::Vertex>,
        index_data: &mut Vec<u32>
    ) {
        let origin = chunk.origin;
        let end_x = origin.x + self.spec.chunk_resolution;
        let end_y = origin.y + self.spec.chunk_resolution;
        for cell_y in origin.y..end_y {
            for cell_x in origin.x..end_x {
                // TEMP: Randomly mutate cell color to make it easier to see edges.
                let root_color = icosahedron::RAINBOW[origin.root.index as usize];
                let mut cell_color = root_color;
                let mut rng = rand::thread_rng();
                for mut color_channel in &mut cell_color {
                    *color_channel *= rng.next_f32();
                }

                // TODO: use functions that return just the bit they care
                // about and... maths. This is silly.
                let first_vertex_index = vertex_data.len() as u32;

                // Output a quad for this cell starting from its
                // center point and going to the next on (x, y).
                // Name anti-clockwise starting from (0, 0).
                // TODO: output hexagons+pentagons instead.
                let a = CellPos {
                    x: cell_x,
                    y: cell_y,
                    root: origin.root,
                };
                let mut b = a;
                b.x += 1;
                let mut c = b;
                c.y += 1;
                let mut d = c;
                d.x -= 1;

                for cell_pos in &[a, b, c, d] {
                    // TEMP: get the height for this cell from chunk data.
                    // TODO: use actual chunk data to render this stuff.
                    //
                    // TODO: don't take a reference to a chunk
                    // in this method; to make geometry for this
                    // chunk we need to have data for adjacent chunks
                    // loaded, and rebase some of the edge positions
                    // on those adjacent chunks to get their cell data.
                    //
                    // **OR** we can have a step before this that
                    // ensures we have all adjacent cell data cached
                    // in extra rows/columns along the edges of this chunk.
                    // The latter probably makes more sense for memory
                    // locality in the hot path. Sometimes we might want
                    // to ask further afield, though, (e.g. five cells
                    // into another chunk) so decide whether you want
                    // a general interface that can fetch as necessary,
                    // commit to always caching as much as you
                    // might ever need, or some combination.
                    //
                    // HAX: For now... hack hack hack. We won't have all
                    // the cell data we need for some edge cells,
                    // so just make it up.
                    use std::cmp::min;
                    let local_x = min(cell_pos.x - origin.x, self.spec.chunk_resolution - 1);
                    let local_y = min(cell_pos.y - origin.y, self.spec.chunk_resolution - 1);
                    let cell_i = (local_y * self.spec.chunk_resolution + local_x) as usize;
                    let cell = &chunk.cells[cell_i];
                    let pt3 = self.cell_center(*cell_pos) * cell.height;

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
                index_data.extend_from_slice(&[
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
        super::project(cell_pos.root, pt_in_root_quad)
    }
}
