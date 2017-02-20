use slog::Logger;

use super::spec::Spec;
use super::{Globe, CellPos, Cursor, ChunkOrigin};
use super::chunk::{ Material };
use super::cell_shape;
use ::render;

// TODO: between this and "draw" we now have some confusing names.
// Shuffle this code into something that implies it's just about
// generating geometry for other components/systems, e.g., drawing
// and physics.

// `View` doesn't store a reference to a `Globe`,
// to avoid complex lifetime wrangling; we might want
// to load and unload globes and their views out of
// step with each other. E.g. we might use a `Globe`
// to create some geometry for a moon, and then never
// use the `Globe` itself again.
//
// Instead, the rendering subsystem will provide us with that
// globe when it wants us to build geometry.
pub struct View {
    spec: Spec,
    log: Logger,
}

impl View {
    pub fn new(globe_spec: Spec, parent_log: &Logger) -> View {
        View {
            spec: globe_spec,
            log: parent_log.new(o!()),
        }
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
        globe: &Globe,
        origin: ChunkOrigin,
        vertex_data: &mut Vec<render::Vertex>,
        index_data: &mut Vec<u32>
    ) {
        trace!(self.log, "Building chunk geometry"; "origin" => format!("{:?}", origin));

        let mut cursor = Cursor::new(globe, *origin.pos());

        // Include cells _on_ the far edge of the chunk;
        // even though we don't own them we'll need to draw part of them.
        let end_x = origin.pos().x + self.spec.chunk_resolution[0];
        let end_y = origin.pos().y + self.spec.chunk_resolution[1];
        // Chunks don't share cells in the z-direction,
        // but do in the x- and y-directions.
        let end_z = origin.pos().z + self.spec.chunk_resolution[2] - 1;
        for cell_z in origin.pos().z..(end_z + 1) {
            for cell_y in origin.pos().y..(end_y + 1) {
                for cell_x in origin.pos().x..(end_x + 1) {
                    // Use cell center as first vertex of each triangle.
                    let cell_pos = CellPos {
                        x: cell_x,
                        y: cell_y,
                        z: cell_z,
                        root: origin.pos().root,
                    };

                    cursor.set_pos(cell_pos);

                    if self.cull_cell(&cursor) {
                       continue;
                    }

                    let mut cell_color = {
                        // Eww... can I please have non-lexical borrow scopes? :)
                        let cell = cursor.cell().expect("We shouldn't be trying to build geometry for a chunk that isn't loaded.");

                        // TEMP color dirt as green, ocean as blue.
                        // TEMP: Randomly mutate cell color to make it easier to see edges.
                        let mut inner_cell_color = if cell.material == Material::Dirt {
                            // Grassy green
                            [ 0.0, 0.4, 0.0 ]
                        } else if cell.material == Material::Water {
                            // Ocean blue
                            [ 0.0, 0.1, 0.7 ]
                        } else {
                            // Don't draw air or anything else we don't understand.
                            continue;
                        };
                        for mut color_channel in &mut inner_cell_color {
                            *color_channel *= 1.0 - 0.5 * cell.shade;
                        }

                        inner_cell_color
                    };

                    // TODO: use functions that return just the bit they care
                    // about and... maths. This is silly.
                    let first_top_vertex_index = vertex_data.len() as u32;

                    // TODO: don't switch; split all this out into calls
                    // over different ranges of cells.
                    //
                    // For now, put the most specific cases first.
                    let cell_shape = if cell_x == 0 && cell_y == 0 {
                        cell_shape::NORTH_PORTION
                    } else if cell_x == end_x && cell_y == end_y {
                        cell_shape::SOUTH_PORTION
                    } else if cell_x == end_x && cell_y == 0 {
                        cell_shape::WEST_PORTION
                    } else if cell_x == 0 && cell_y == end_y {
                        cell_shape::EAST_PORTION
                    } else if cell_y == 0 {
                        cell_shape::NORTH_WEST_PORTION
                    } else if cell_x == 0 {
                        cell_shape::NORTH_EAST_PORTION
                    } else if cell_x == end_x {
                        cell_shape::SOUTH_WEST_PORTION
                    } else if cell_y == end_y {
                        cell_shape::SOUTH_EAST_PORTION
                    } else {
                        cell_shape::FULL_HEX
                    };

                    // Emit each top vertex of whatever shape we're using for this cell.
                    let offsets = &cell_shape.top_outline_dir_offsets;
                    for offset in offsets.iter() {
                        let vertex_pt3 = self.spec.cell_top_vertex(cell_pos, *offset);
                        vertex_data.push(render::Vertex::new([
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
                        let vertex_pt3 = self.spec.cell_top_vertex(cell_pos, *offset);
                        vertex_data.push(render::Vertex::new([
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
                        let vertex_pt3 = self.spec.cell_bottom_vertex(cell_pos, *offset);
                        vertex_data.push(render::Vertex::new([
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

    fn cull_cell(&self, cursor: &Cursor) -> bool {
        use super::Neighbors;

        let resolution = cursor.globe().spec().root_resolution;

        let cell_pos = cursor.pos();
        let mut neighbor_cursor = cursor.clone();

        // If none of the neighboring cells contain air,
        // then we won't render the cell at all.
        let neighbors = Neighbors::new(cell_pos, resolution);
        for neighbor_pos in neighbors {
            neighbor_cursor.set_pos(neighbor_pos);
            if let Some(neighbor) = neighbor_cursor.cell() {
                if neighbor.material == Material::Air {
                    // This cell can be seen; we can't cull it.
                    return false;
                }
            }
        }

        // If there was no reason to save it,
        // then assume we can cull it!
        true
    }
}
