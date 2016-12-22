use types::*;

use super::IntCoord;
use super::CellPos;

// Contains the specifications (dimensions, seed, etc.)
// needed to deterministically generate a `Globe`.
//
// Provides helper functions that don't need to know anything
// beyond these values.
//
// TODO: accessors for all the fields, and make them private.
//
// TODO: split out parameters that are applicable to all
// kinds of globes, and those specific to individual kinds
// of globes.
#[derive(Clone, Copy)]
pub struct Spec {
    pub seed: u32,
    pub floor_radius: f64,
    pub ocean_radius: f64,
    pub block_height: f64,
    // These are the full width/height/depth of a given root quad or chunk's voxmap;
    // i.e. not an exponent. Only chunks specify a depth resolution because the
    // world can have unbounded total depth.
    pub root_resolution: [IntCoord; 2],
    pub chunk_resolution: [IntCoord; 3],
}

impl Spec {
    pub fn is_valid(&self) -> bool {
        // Chunk resolution needs to divide perfectly into root resolution.
        let cprs = self.chunks_per_root_side();
        let calculated_root_resolution = [
            cprs[0] * self.chunk_resolution[0],
            cprs[1] * self.chunk_resolution[1],
        ];
        if calculated_root_resolution != self.root_resolution {
            return false;
        }

        // Root resolution needs to be exactly twice in the y-direction
        // that it is in the x-direction. I can't think of any serious
        // use cases for anything else, and it's extremely unclear how
        // a lot of scenarios should work otherwise.
        if self.root_resolution[1] != self.root_resolution[0] * 2 {
            return false;
        }

        return true;
    }

    pub fn chunks_per_root_side(&self) -> [IntCoord; 2] {
        // Assume chunk resolution divides perfectly into root resolution.
        [
            self.root_resolution[0] / self.chunk_resolution[0],
            self.root_resolution[1] / self.chunk_resolution[1],
        ]
    }

    // Ignore the z-coordinate; just project to a unit sphere.
    // This is useful for, e.g., sampling noise to determine elevation
    // at a particular point on the surface, or other places where you're
    // really just talking about longitude/latitude.
    pub fn cell_center_on_unit_sphere(&self, cell_pos: CellPos) -> Pt3 {
        let res_x = self.root_resolution[0] as f64;
        let res_y = self.root_resolution[1] as f64;
        let pt_in_root_quad = Pt2::new(
            cell_pos.x as f64 / res_x,
            cell_pos.y as f64 / res_y,
        );
        super::project(cell_pos.root, pt_in_root_quad)
    }

    pub fn cell_center_center(&self, cell_pos: CellPos) -> Pt3 {
        let radius = self.floor_radius +
            self.block_height * (cell_pos.z as f64 + 0.5);
        radius * self.cell_center_on_unit_sphere(cell_pos)
    }

    // TODO: describe meaning of offsets, where to get it from, etc.?
    pub fn cell_vertex_on_unit_sphere(&self, cell_pos: CellPos, offset: [i64; 2]) -> Pt3 {
        let res_x = (self.root_resolution[0] * 6) as f64;
        let res_y = (self.root_resolution[1] * 6) as f64;
        let pt_in_root_quad = Pt2::new(
            (cell_pos.x as i64 * 6 + offset[0]) as f64 / res_x,
            (cell_pos.y as i64 * 6 + offset[1]) as f64 / res_y,
        );
        super::project(cell_pos.root, pt_in_root_quad)
    }

    pub fn cell_bottom_vertex(&self, cell_pos: CellPos, offset: [i64; 2]) -> Pt3 {
        let radius = self.floor_radius +
            self.block_height * cell_pos.z as f64;
        radius * self.cell_vertex_on_unit_sphere(cell_pos, offset)
    }

    pub fn cell_top_vertex(&self, mut cell_pos: CellPos, offset: [i64; 2]) -> Pt3 {
        // The top of one cell is the bottom of the next.
        cell_pos.z += 1;
        self.cell_bottom_vertex(cell_pos, offset)
    }

    // Returns a position equivalent to `pos`,
    // but in whatever root owns the data for `pos`.
    //
    // The output will only ever differ from the input
    // if `pos` is on the edge of a root quad.
    //
    // Will return nonsense (or panics) if `pos` lies beyond the
    // edges of its root.
    pub fn pos_in_owning_root(&self, pos: CellPos) -> CellPos {
        // Here is the pattern of which root a cell belongs to.
        //
        // Note how adacent roots neatly slot into each other's
        // non-owned cells when wrapped around the globe.
        //
        // Also note the special cases for north and south poles;
        // they don't fit neatly into the general pattern.
        //
        // In the diagram below, each circle represents a hexagon
        // in a voxmap shell. Filled circles belong to the root,
        // and empty circles belong to an adjacent root.
        //
        //   Root 0   Roots 1, 2, 3   Root 4
        //   ------   -------------   ------
        //
        //      ●           ◌           ◌
        //     ◌ ●         ◌ ●         ◌ ●
        //    ◌ ● ●       ◌ ● ●       ◌ ● ●
        //   ◌ ● ● ●     ◌ ● ● ●     ◌ ● ● ●
        //  ◌ ● ● ● ●   ◌ ● ● ● ●   ◌ ● ● ● ●
        //   ◌ ● ● ● ●   ◌ ● ● ● ●   ◌ ● ● ● ●
        //    ◌ ● ● ● ●   ◌ ● ● ● ●   ◌ ● ● ● ●
        //     ◌ ● ● ● ●   ◌ ● ● ● ●   ◌ ● ● ● ●
        //      ◌ ● ● ● ●   ◌ ● ● ● ●   ◌ ● ● ● ●
        //       ◌ ● ● ●     ◌ ● ● ●     ◌ ● ● ●
        //        ◌ ● ●       ◌ ● ●       ◌ ● ●
        //         ◌ ●         ◌ ●         ◌ ●
        //          ◌           ◌           ●
        //
        let end_x = self.root_resolution[0];
        let end_y = self.root_resolution[1];
        let half_y = self.root_resolution[1] / 2;

        // Special cases for north and south poles
        if pos.x == 0 && pos.y == 0 {
            // North pole
            CellPos {
                // First root owns north pole.
                root: 0.into(),
                x: 0,
                y: 0,
                z: pos.z,
            }
        } else if pos.x == end_x && pos.y == end_y {
            // South pole
            CellPos {
                // Last root owns south pole.
                root: 4.into(),
                x: end_x,
                y: end_y,
                z: pos.z,
            }
        } else if pos.y == 0 {
            // Roots don't own their north-west edge;
            // translate to next root's north-east edge.
            CellPos {
                root: pos.root.next_west(),
                x: 0,
                y: pos.x,
                z: pos.z,
            }
        } else if pos.x == end_x && pos.y < half_y {
            // Roots don't own their mid-west edge;
            // translate to the next root's mid-east edge.
            CellPos {
                root: pos.root.next_west(),
                x: 0,
                y: half_y + pos.y,
                z: pos.z,
            }
        } else if pos.x == end_x {
            // Roots don't own their south-west edge;
            // translate to the next root's south-east edge.
            CellPos {
                root: pos.root.next_west(),
                y: end_y,
                x: pos.y - half_y,
                z: pos.z,
            }
        } else {
            // `pos` is either on an edge owned by its root,
            // or somewhere in the middle of the root.
            pos
        }
    }
}
