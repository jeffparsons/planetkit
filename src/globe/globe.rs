use rand;
use rand::Rng;

use noise;

use types::*;
use super::Root;
use super::chunk::{ Chunk, CellPos, Cell, Material };
use super::spec::Spec;
use super::cell_shape;

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
                floor_radius: 0.91, // TODO: make it ~Earth
                // NOTE: Don't let ocean radius be a neat multiple of block
                // height above floor radius, or we'll end up with
                // z-fighting in evaluating what blocks are water/air.
                ocean_radius: 1.13,
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
            for z in 0..5 {
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
        // Include cells _on_ the far edge of the chunk;
        // even though we don't own them we'll need to draw part of them.
        let end_x = origin.x + self.spec.chunk_resolution[0] + 1;
        let end_y = origin.y + self.spec.chunk_resolution[1] + 1;
        let end_z = origin.z + self.spec.chunk_resolution[2] + 1;
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
                    let cell_pt3 = self.cell_center_center(cell_pos);

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

    // TODO: don't take a reference to a chunk
    // in this method; to make geometry for this
    // chunk we'll eventually need to have data for adjacent chunks
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
    pub fn make_chunk_geometry(
        &self,
        chunk: &Chunk,
        vertex_data: &mut Vec<::Vertex>,
        index_data: &mut Vec<u32>
    ) {
        let origin = chunk.origin;
        // Include cells _on_ the far edge of the chunk;
        // even though we don't own them we'll need to draw part of them.
        let end_x = origin.x + self.spec.chunk_resolution[0] + 1;
        let end_y = origin.y + self.spec.chunk_resolution[1] + 1;
        let end_z = origin.z + self.spec.chunk_resolution[2] + 1;
        for cell_z in origin.z..end_z {
            for cell_y in origin.y..end_y {
                for cell_x in origin.x..end_x {
                    // Use cell centre as first vertex of each triangle.
                    let cell_pos = CellPos {
                        x: cell_x,
                        y: cell_y,
                        z: cell_z,
                        root: origin.root,
                    };
                    let cell = chunk.cell(cell_pos);

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
                    let first_top_vertex_index = vertex_data.len() as u32;

                    // Emit each top vertex of whatever shape we're using for this cell.
                    let offsets = &cell_shape::FULL_HEX.top_outline_dir_offsets;
                    for offset in offsets.iter() {
                        let vertex_pt3 = self.cell_top_vertex(cell_pos, *offset);
                        vertex_data.push(::Vertex::new([
                            vertex_pt3[0] as f32,
                            vertex_pt3[1] as f32,
                            vertex_pt3[2] as f32,
                        ], cell_color));
                    }

                    // Emit triangles for the top of the cell. All triangles
                    // will contain the first vertex, plus two others.
                    for i in 1..(offsets.len() as u32 - 1) {
                        index_data.extend_from_slice(&[
                            first_top_vertex_index,
                            first_top_vertex_index + i,
                            first_top_vertex_index + i + 1,
                        ]);
                    }

                    // Emit each top vertex of whatever shape we're using for this cell
                    // AGAIN for the top of the sides, so they can have a different colour.
                    // Darken the top of the sides slightly to fake lighting.
                    for mut color_channel in &mut cell_color {
                        *color_channel *= 0.9;
                    }
                    let first_side_top_vertex_index = first_top_vertex_index
                        + offsets.len() as u32;
                    for offset in offsets.iter() {
                        let vertex_pt3 = self.cell_top_vertex(cell_pos, *offset);
                        vertex_data.push(::Vertex::new([
                            vertex_pt3[0] as f32,
                            vertex_pt3[1] as f32,
                            vertex_pt3[2] as f32,
                        ], cell_color));
                    }

                    // Emit each bottom vertex of whatever shape we're using for this cell.
                    // Darken the bottom of the sides substantially to fake lighting.
                    for mut color_channel in &mut cell_color {
                        *color_channel *= 0.5;
                    }
                    let first_side_bottom_vertex_index = first_side_top_vertex_index
                        + offsets.len() as u32;
                    for offset in offsets.iter() {
                        let vertex_pt3 = self.cell_bottom_vertex(cell_pos, *offset);
                        vertex_data.push(::Vertex::new([
                            vertex_pt3[0] as f32,
                            vertex_pt3[1] as f32,
                            vertex_pt3[2] as f32,
                        ], cell_color));
                    }

                    // Emit triangles for the cell sides.
                    for ab_i in 0..(offsets.len() as u32) {
                        let cd_i = (ab_i + 1) % offsets.len() as u32;
                        let a_i = first_side_top_vertex_index + ab_i;
                        let b_i = first_side_bottom_vertex_index + ab_i;
                        let c_i = first_side_bottom_vertex_index + cd_i;
                        let d_i = first_side_top_vertex_index + cd_i;
                        index_data.extend_from_slice(&[
                            a_i, b_i, d_i,
                            d_i, b_i, c_i,
                        ]);
                    }
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

    fn cell_center_center(&self, cell_pos: CellPos) -> Pt3 {
        let radius = self.spec.floor_radius +
            self.spec.block_height * (cell_pos.z as f64 + 0.5);
        radius * self.cell_center_on_unit_sphere(cell_pos)
    }

    fn cell_bottom_center(&self, cell_pos: CellPos) -> Pt3 {
        let radius = self.spec.floor_radius +
            self.spec.block_height * cell_pos.z as f64;
        radius * self.cell_center_on_unit_sphere(cell_pos)
    }

    fn cell_top_center(&self, mut cell_pos: CellPos) -> Pt3 {
        // The top of one cell is the bottom of the next.
        cell_pos.z += 1;
        self.cell_bottom_center(cell_pos)
    }

    // TODO: describe meaning of offsets, where to get it from, etc.?
    fn cell_vertex_on_unit_sphere(&self, cell_pos: CellPos, offset: [i64; 2]) -> Pt3 {
        let res_x = (self.spec.root_resolution[0] * 6) as f64;
        let res_y = (self.spec.root_resolution[1] * 6) as f64;
        let pt_in_root_quad = Pt2::new(
            (cell_pos.x as i64 * 6 + offset[0]) as f64 / res_x,
            (cell_pos.y as i64 * 6 + offset[1]) as f64 / res_y,
        );
        super::project(cell_pos.root, pt_in_root_quad)
    }

    fn cell_bottom_vertex(&self, cell_pos: CellPos, offset: [i64; 2]) -> Pt3 {
        let radius = self.spec.floor_radius +
            self.spec.block_height * cell_pos.z as f64;
        radius * self.cell_vertex_on_unit_sphere(cell_pos, offset)
    }

    fn cell_top_vertex(&self, mut cell_pos: CellPos, offset: [i64; 2]) -> Pt3 {
        // The top of one cell is the bottom of the next.
        cell_pos.z += 1;
        self.cell_bottom_vertex(cell_pos, offset)
    }
}
