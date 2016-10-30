use rand;
use rand::Rng;

use noise;

use types::*;
use super::{ Root, Dir };
use super::chunk::{ Chunk, CellPos, Cell, Material };
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
                floor_radius: 0.90, // TODO: make it ~Earth
                // NOTE: Don't let ocean radius be a neat multiple of block
                // height above floor radius, or we'll end up with
                // z-fighting in evaluating what blocks are water/air.
                ocean_radius: 1.01,
                block_height: 0.02,
                root_resolution: [32, 32],
                chunk_resolution: [16, 16, 4],
            }
        )
    }

    pub fn build_all_chunks(&mut self) {
        // Calculate how many chunks to a root in each direction in (x, y).
        let chunks_per_root = [
            self.spec.root_resolution[0] / self.spec.chunk_resolution[0],
            self.spec.root_resolution[1] / self.spec.chunk_resolution[1],
        ];
        for root_index in 0..ROOT_QUADS {
            let root = Root { index: root_index };
            // TODO: how many to build high?
            for z in 0..3 {
                for y in 0..chunks_per_root[0] {
                    for x in 0..chunks_per_root[1] {
                        let origin = CellPos {
                            root: root,
                            x: x * self.spec.chunk_resolution[0],
                            y: y * self.spec.chunk_resolution[1],
                            z: z * self.spec.chunk_resolution[2],
                        };
                        self.build_chunk(origin);
                    }
                }
            }
        }
    }

    pub fn build_chunk(&mut self, origin: CellPos) {
        // TODO: get parameters from spec
        let noise = noise::Brownian3::new(noise::open_simplex3::<f64>, 6).wavelength(1.0);
        let mut cells: Vec<Cell> = Vec::new();
        let end_x = origin.x + self.spec.chunk_resolution[0];
        let end_y = origin.y + self.spec.chunk_resolution[1];
        let end_z = origin.z + self.spec.chunk_resolution[2];
        for cell_z in origin.z..end_z {
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
                        z: cell_z,
                    };
                    let land_pt3 = self.cell_center_on_unit_sphere(cell_pos);
                    let cell_pt3 = self.cell_center(cell_pos);

                    // Vary a little bit around 1.0.
                    let delta =
                        noise.apply(&self.pt, land_pt3.as_ref())
                        * self.spec.ocean_radius
                        * 0.3;
                    let land_height = self.spec.ocean_radius + delta;
                    // TEMP: ...
                    use na::Norm;
                    let cell_height = cell_pt3.as_vector().norm();
                    let material = if cell_height < land_height {
                        Material::Dirt
                    } else if cell_height < self.spec.ocean_radius {
                        Material::Water
                    } else {
                        Material::Air
                    };
                    cells.push(Cell {
                        material: material,
                    });
                }
            }
        }
        self.chunks.push(Chunk {
            origin: origin,
            cells: cells,
            resolution: self.spec.chunk_resolution,
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
        let end_x = origin.x + self.spec.chunk_resolution[0];
        let end_y = origin.y + self.spec.chunk_resolution[1];
        let end_z = origin.z + self.spec.chunk_resolution[2];
        for cell_z in origin.z..end_z {
            for cell_y in origin.y..end_y {
                for cell_x in origin.x..end_x {
                    // TEMP: only draw this cell if it's dirt.
                    // Use cell centre as top-left corner of quad.
                    let a = CellPos {
                        x: cell_x,
                        y: cell_y,
                        z: cell_z,
                        root: origin.root,
                    };
                    let cell = chunk.cell(a);

                    // TEMP color dirt as green, ocean as blue.
                    // TEMP: Randomly mutate cell color to make it easier to see edges.
                    let mut cell_color = if cell.material == Material::Dirt {
                        // Grassy green
                        [ 0.0, 0.4, 0.0 ]
                    } else if cell.material == Material::Water {
                        // Ocean blue
                        [ 0.0, 0.1, 0.7 ]
                    } else {
                        // Don't draw air or anything else we don't understand.
                        continue;
                    };
                    let mut rng = rand::thread_rng();
                    for mut color_channel in &mut cell_color {
                        *color_channel *= 1.0 - 0.5 * rng.next_f32();
                    }

                    // TODO: use functions that return just the bit they care
                    // about and... maths. This is silly.
                    let first_vertex_index = vertex_data.len() as u32;

                    // Output a quad for this cell starting from its
                    // center point and going to the next on (x, y).
                    // Name anti-clockwise starting from (0, 0).
                    // TODO: output hexagons+pentagons instead.
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
    }

    // Ignore the z-coordinate; just project to a unit sphere.
    // This is useful for, e.g., sampling noise to determine elevation
    // at a particular point on the surface, or other places where you're
    // really just talking about longitude/latitude.
    fn cell_center_on_unit_sphere(&self, cell_pos: CellPos) -> Pt3 {
        let res_x = self.spec.root_resolution[0] as f64;
        let res_y = self.spec.root_resolution[1] as f64;
        let pt_in_root_quad = Pt2::new(
            cell_pos.x as f64 / res_x,
            cell_pos.y as f64 / res_y,
        );
        super::project(cell_pos.root, pt_in_root_quad)
    }

    fn cell_center(&self, cell_pos: CellPos) -> Pt3 {
        let radius = self.spec.floor_radius +
            self.spec.block_height * cell_pos.z as f64;
        radius * self.cell_center_on_unit_sphere(cell_pos)
    }

    // TODO: return something
    fn cell_vertex(&self, cell_pos: CellPos, dir: Dir) {
        // We can imagine a hexagon laid out on a quad
        // that wraps in both directions, such that its
        // center exists at all four corners of the quad:
        //
        //  (0, 0)
        //          ●       y _
        //    x        ◌       🡖
        //    🡓     ◌     ◌
        //             ◌     ●
        //          ◌     ◌ ╱   ◌
        //             ◌   ╱ ◌     ◌     (0, 0)
        //          ●─────●     ◌     ●
        //             ◌   ╲ ◌     ◌
        //          ◌     ◌ ╲   ◌     ◌
        //             ◌     ●     ◌
        //          ◌     ◌   ╲ ◌     ◌
        //             ◌     ◌ ╲   ◌
        //          ●     ◌     ●─────●
        //  (0, 0)     ◌     ◌ ╱   ◌
        //                ◌   ╱ ◌     ◌
        //                   ●     ◌
        //                      ◌     ◌
        //                         ◌
        //                            ●
        //                               (0, 0)
        //
        // Then, if we list out points for the middle of
        // each side and each vertex, starting from the
        // middle of the side in the positive x direction
        // and travelling counterclockwise, we end up with
        // 12 offset coordinate pairs in this grid of:
        const DIR_OFFSETS: [[i8; 2]; 12] = [
            [ 3,  0], // edge (+x)
            [ 2,  2], // vertex
            [ 0,  3], // edge (+y)
            [-2,  4], // vertex
            [-3,  3], // edge
            [-4,  2], // vertex
            [-3,  0], // edge (-x)
            [-2, -2], // vertex
            [ 0, -3], // edge (-y)
            [ 2, -4], // vertex
            [ 3, -3], // edge
            [ 4, -2], // vertex
        ];
    }
}
