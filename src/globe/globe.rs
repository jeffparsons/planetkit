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
                    let mut first_vertex_index = vertex_data.len() as u32;

                    // Output triangles made of the cell center and
                    // each pair of adjacent vertices of the hexagon.
                    // See `cell_vertex_on_unit_sphere` below for explanation
                    // of the directions we've chosen here.
                    //
                    // TODO: we shouldn't be drawing the full hexagon for all cells;
                    // otherwise adjacent chunks will both be trying to draw the same
                    // hexagon at the edges. Instead we should determine which
                    // vertices we need, and draw partial hexagons like so:
                    //
                    //                 6
                    //        7                 5
                    //           ◌ · · ●─────●
                    //          ·   ◌  │  ◌   \
                    //         · ◌     ◌     ◌ \
                    //     8  ◌     ◌  │  ◌     ◌  4
                    //       ·   ◌     ◌     ◌   \
                    //      · ◌     ◌  │  ◌     ◌ \
                    //  9  ◌     ◌     ●     ◌     ●  3
                    //      · ◌     ◌  │  ◌     ◌ /
                    //       ·   ◌     ◌     ◌   /
                    //        ◌     ◌  │  ◌     ◌
                    //    10   · ◌     ◌     ◌ /   2
                    //          ·   ◌  │  ◌   /         y
                    //           ◌ · · ●─────●           ↘
                    //       11                 1
                    //                 0
                    //
                    //                 x
                    //                 ↓
                    //
                    let top_center_pt3 = self.cell_top_center(cell_pos);
                    let top_vertices = [
                        self.cell_top_vertex(cell_pos, Dir(1)),
                        self.cell_top_vertex(cell_pos, Dir(3)),
                        self.cell_top_vertex(cell_pos, Dir(5)),
                        self.cell_top_vertex(cell_pos, Dir(7)),
                        self.cell_top_vertex(cell_pos, Dir(9)),
                        self.cell_top_vertex(cell_pos, Dir(11)),
                    ];
                    for a_i in 0..6 {
                        let b_i = (a_i + 1) % 6;
                        let a_pt3 = top_vertices[a_i];
                        let b_pt3 = top_vertices[b_i];
                        // Push vertex for hexagon center.
                        vertex_data.push(::Vertex::new([
                            top_center_pt3[0] as f32,
                            top_center_pt3[1] as f32,
                            top_center_pt3[2] as f32,
                        ], cell_color));
                        // Push vertex for hexagon vertex 'a'.
                        vertex_data.push(::Vertex::new([
                            a_pt3[0] as f32,
                            a_pt3[1] as f32,
                            a_pt3[2] as f32,
                        ], cell_color));
                        // Push vertex for hexagon vertex 'b'.
                        vertex_data.push(::Vertex::new([
                            b_pt3[0] as f32,
                            b_pt3[1] as f32,
                            b_pt3[2] as f32,
                        ], cell_color));
                    }

                    // Output all six triangle faces for the cell.
                    let mut vertex_indices: Vec<u32> =
                        (first_vertex_index..(first_vertex_index + 6*3))
                        .collect();
                    index_data.extend_from_slice(&vertex_indices);
                    // Yes indeed, this is rather a lot of hacks.
                    first_vertex_index += 6*3;

                    // Now output the vertices for the cell sides.
                    // Darken the sides substantially to fake lighting.
                    for mut color_channel in &mut cell_color {
                        *color_channel *= 0.5;
                    }
                    let bottom_vertices = [
                        self.cell_bottom_vertex(cell_pos, Dir(1)),
                        self.cell_bottom_vertex(cell_pos, Dir(3)),
                        self.cell_bottom_vertex(cell_pos, Dir(5)),
                        self.cell_bottom_vertex(cell_pos, Dir(7)),
                        self.cell_bottom_vertex(cell_pos, Dir(9)),
                        self.cell_bottom_vertex(cell_pos, Dir(11)),
                    ];
                    for ab_i in 0..6 {
                        let cd_i = (ab_i + 1) % 6;
                        let a_pt3 = top_vertices[ab_i];
                        let b_pt3 = bottom_vertices[ab_i];
                        let c_pt3 = bottom_vertices[cd_i];
                        let d_pt3 = top_vertices[cd_i];
                        let quad_triangle_vertices = [
                            a_pt3, b_pt3, d_pt3,
                            d_pt3, b_pt3, c_pt3,
                        ];
                        for quad_triangle_vertex in &quad_triangle_vertices {
                            vertex_data.push(::Vertex::new([
                                quad_triangle_vertex[0] as f32,
                                quad_triangle_vertex[1] as f32,
                                quad_triangle_vertex[2] as f32,
                            ], cell_color));
                        }
                    }

                    // Output triangles for 6 triangles * 2 triangles * 3 vertices each.
                    let new_vertex_indices: Vec<u32> =
                        (first_vertex_index..(first_vertex_index + 6*2*3))
                        .collect();
                    vertex_indices.extend_from_slice(&new_vertex_indices);
                    index_data.extend_from_slice(&vertex_indices);
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

    fn cell_vertex_on_unit_sphere(&self, cell_pos: CellPos, dir: Dir) -> Pt3 {
        // We can imagine a hexagon laid out on a quad
        // that wraps in both directions, such that its
        // center exists at all four corners of the quad:
        //
        //  (0, 0)
        //          ●       y
        //    x        ◌      ↘
        //    ↓     ◌     ◌
        //             ◌     ●
        //          ◌     ◌ /   ◌
        //             ◌   / ◌     ◌     (0, 0)
        //          ●─────●     ◌     ●
        //             ◌   \ ◌     ◌
        //          ◌     ◌ \   ◌     ◌
        //             ◌     ●     ◌
        //          ◌     ◌   \ ◌     ◌
        //             ◌     ◌ \   ◌
        //          ●     ◌     ●─────●
        //  (0, 0)     ◌     ◌ /   ◌
        //                ◌   / ◌     ◌
        //                   ●     ◌
        //                      ◌     ◌
        //                         ◌
        //                            ●
        //                               (0, 0)
        //
        // This makes it visually obvious that we're dealing
        // with a grid of 6 units between hexagon centers (count it)
        // to calculate cell vertex positions (if we want all vertices
        // to lie at integer coordinate pairs) as opposed to the 1 unit
        // between cell centers when we're only concerned with the
        // center points of each cell.
        //
        // Then, if we list out points for the middle of
        // each side and each vertex, starting from the
        // middle of the side in the positive x direction
        // and travelling counterclockwise, we end up with
        // 12 offset coordinate pairs in this grid, labelled as follows:
        //
        //                 6
        //        7                 5
        //           ●─────●─────●
        //          /   ◌     ◌   \
        //         / ◌     ◌     ◌ \
        //     8  ●     ◌     ◌     ●  4
        //       /   ◌     ◌     ◌   \
        //      / ◌     ◌     ◌     ◌ \
        //  9  ●     ◌     ●     ◌     ●  3
        //      \ ◌     ◌     ◌     ◌ /
        //       \   ◌     ◌     ◌   /
        //        ●     ◌     ◌     ●
        //    10   \ ◌     ◌     ◌ /   2
        //          \   ◌     ◌   /         y
        //           ●─────●─────●           ↘
        //       11                 1
        //                 0
        //
        //                 x
        //                 ↓
        //
        // Referring to the top figure for the offsets and the
        // bottom for the labelling, that gives us:
        const DIR_OFFSETS: [[i64; 2]; 12] = [
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
        let res_x = (self.spec.root_resolution[0] * 6) as f64;
        let res_y = (self.spec.root_resolution[1] * 6) as f64;
        let pt_in_root_quad = Pt2::new(
            (cell_pos.x as i64 * 6 + DIR_OFFSETS[dir.0 as usize][0]) as f64 / res_x,
            (cell_pos.y as i64 * 6 + DIR_OFFSETS[dir.0 as usize][1]) as f64 / res_y,
        );
        super::project(cell_pos.root, pt_in_root_quad)
    }

    fn cell_bottom_vertex(&self, cell_pos: CellPos, dir: Dir) -> Pt3 {
        let radius = self.spec.floor_radius +
            self.spec.block_height * cell_pos.z as f64;
        radius * self.cell_vertex_on_unit_sphere(cell_pos, dir)
    }

    fn cell_top_vertex(&self, mut cell_pos: CellPos, dir: Dir) -> Pt3 {
        // The top of one cell is the bottom of the next.
        cell_pos.z += 1;
        self.cell_bottom_vertex(cell_pos, dir)
    }
}
