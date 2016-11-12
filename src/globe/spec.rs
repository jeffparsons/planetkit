use super::IntCoord;

// TODO: accessors for all the fields, and make them private.
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
        calculated_root_resolution == self.root_resolution
    }

    pub fn chunks_per_root_side(&self) -> [IntCoord; 2] {
        // Assume chunk resolution divides perfectly into root resolution.
        [
            self.root_resolution[0] / self.chunk_resolution[0],
            self.root_resolution[1] / self.chunk_resolution[1],
        ]
    }
}
