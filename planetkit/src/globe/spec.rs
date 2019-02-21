use crate::types::*;

use crate::grid::{GridCoord, GridPoint2, Point3};

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
    pub seed: u64,
    // `rand` consumes PRNG seeds from a `[u8; 16]`,
    // so we store it as that (big-endian) as well for convenience.
    pub seed_as_u8_array: [u8; 16],
    pub floor_radius: f64,
    // NOTE: Don't let ocean radius be a neat multiple of block
    // height above floor radius, or we'll end up with
    // z-fighting in evaluating what blocks are water/air.
    pub ocean_radius: f64,
    pub block_height: f64,
    // These are the full width/height/depth of a given root quad or chunk's voxmap;
    // i.e. not an exponent. Only chunks specify a depth resolution because the
    // world can have unbounded total depth.
    pub root_resolution: [GridCoord; 2],
    pub chunk_resolution: [GridCoord; 3],
}

impl Spec {
    // TODO: Replace with builder pattern.
    pub fn new(
        // TODO: bump up to u128 when `bytes` releases
        // support for that.
        seed: u64,
        floor_radius: f64,
        ocean_radius: f64,
        block_height: f64,
        root_resolution: [GridCoord; 2],
        chunk_resolution: [GridCoord; 3],
    ) -> Spec {
        // Convert seed to `u8` array for convenience when providing
        // it to `rand`.
        use bytes::{BigEndian, ByteOrder};
        let mut seed_as_u8_array = [0u8; 16];
        // TODO: Just writing the seed twice for now;
        // will use an actual `u128` seed when `bytes` supports
        // writing that.
        BigEndian::write_u64(&mut seed_as_u8_array[..8], seed);
        BigEndian::write_u64(&mut seed_as_u8_array[8..], seed);

        Spec {
            seed: seed,
            seed_as_u8_array: seed_as_u8_array,
            floor_radius: floor_radius,
            ocean_radius: ocean_radius,
            block_height: block_height,
            root_resolution: root_resolution,
            chunk_resolution: chunk_resolution,
        }
    }

    pub fn new_earth_scale_example() -> Spec {
        // TODO: This only coincidentally puts you on land.
        // Implement deterministic (PRNG) land finding so that the seed does not matter.
        //
        // ^^ TODO: Is this still true? I think this is actually implemented now.
        let seed = 14;

        let ocean_radius = 6_371_000.0;
        // TODO: actually more like 60_000 when we know how to:
        // - Unload chunks properly
        // - Start with a guess about the z-position of the player
        //   so we don't have to start at bedrock and search up.
        let crust_depth = 60.0;
        let floor_radius = ocean_radius - crust_depth;
        Spec::new(
            seed,
            floor_radius,
            ocean_radius,
            0.65,
            [8388608, 16777216],
            // Chunks should probably be taller, but short chunks are a bit
            // better for now in exposing bugs visually.
            [16, 16, 4],
        )
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

    pub fn chunks_per_root_side(&self) -> [GridCoord; 2] {
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
    pub fn cell_center_on_unit_sphere(&self, column: GridPoint2) -> Pt3 {
        let res_x = self.root_resolution[0] as f64;
        let res_y = self.root_resolution[1] as f64;
        let pt_in_root_quad = Pt2::new(column.x as f64 / res_x, column.y as f64 / res_y);
        super::project(column.root, pt_in_root_quad)
    }

    pub fn cell_center_center(&self, grid_point: Point3) -> Pt3 {
        let radius = self.floor_radius + self.block_height * (grid_point.z as f64 + 0.5);
        radius * self.cell_center_on_unit_sphere(grid_point.rxy)
    }

    pub fn cell_bottom_center(&self, grid_point: Point3) -> Pt3 {
        let radius = self.floor_radius + self.block_height * (grid_point.z as f64);
        radius * self.cell_center_on_unit_sphere(grid_point.rxy)
    }

    // TODO: describe meaning of offsets, where to get it from, etc.?
    pub fn cell_vertex_on_unit_sphere(&self, grid_point: Point3, offset: [i64; 2]) -> Pt3 {
        let res_x = (self.root_resolution[0] * 6) as f64;
        let res_y = (self.root_resolution[1] * 6) as f64;
        let pt_in_root_quad = Pt2::new(
            (grid_point.x as i64 * 6 + offset[0]) as f64 / res_x,
            (grid_point.y as i64 * 6 + offset[1]) as f64 / res_y,
        );
        super::project(grid_point.root, pt_in_root_quad)
    }

    pub fn cell_bottom_vertex(&self, grid_point: Point3, offset: [i64; 2]) -> Pt3 {
        let radius = self.floor_radius + self.block_height * grid_point.z as f64;
        radius * self.cell_vertex_on_unit_sphere(grid_point, offset)
    }

    pub fn cell_top_vertex(&self, mut grid_point: Point3, offset: [i64; 2]) -> Pt3 {
        // The top of one cell is the bottom of the next.
        grid_point.z += 1;
        self.cell_bottom_vertex(grid_point, offset)
    }

    // TODO: test me.
    pub fn approx_cell_z_from_radius(&self, radius: f64) -> GridCoord {
        ((radius - self.floor_radius) / self.block_height) as GridCoord
    }
}
