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
    // NOTE: Don't let ocean radius be a neat multiple of block
    // height above floor radius, or we'll end up with
    // z-fighting in evaluating what blocks are water/air.
    pub ocean_radius: f64,
    pub block_height: f64,
    // These are the full width/height/depth of a given root quad or chunk's voxmap;
    // i.e. not an exponent. Only chunks specify a depth resolution because the
    // world can have unbounded total depth.
    pub root_resolution: [IntCoord; 2],
    pub chunk_resolution: [IntCoord; 3],
}

impl Spec {
    pub fn new_earth_scale_example() -> Spec {
        let ocean_radius = 6_371_000.0;
        // TODO: actually more like 60_000 when we know how to:
        // - Unload chunks properly
        // - Start with a guess about the z-position of the player
        //   so we don't have to start at bedrock and search up.
        let crust_depth = 60.0;
        let floor_radius = ocean_radius - crust_depth;
        Spec {
            // TODO: This only coincidentally puts you on land.
            // Implement deterministic (PRNG) land finding so that the seed does not matter.
            seed: 14,
            floor_radius: floor_radius,
            ocean_radius: ocean_radius,
            block_height: 0.65,
            root_resolution: [8388608, 16777216],
            // Chunks should probably be taller, but short chunks are a bit
            // better for now in exposing bugs visually.
            chunk_resolution: [16, 16, 4],
        }
    }

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

        true
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

    pub fn cell_bottom_center(&self, cell_pos: CellPos) -> Pt3 {
        let radius = self.floor_radius +
            self.block_height * (cell_pos.z as f64);
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

    // TODO: test me.
    pub fn approx_cell_z_from_radius(&self, radius: f64) -> IntCoord {
        ((radius - self.floor_radius) / self.block_height) as IntCoord
    }
}
