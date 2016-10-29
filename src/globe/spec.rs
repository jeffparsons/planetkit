use super::IntCoord;

// TODO: accessors for all the fields, and make them private.
pub struct Spec {
    pub seed: u32,
    pub radius: f64,
    // These are the full width/height of a given root quad or chunk's voxmap;
    // i.e. not an exponent.
    pub root_resolution: IntCoord,
    pub chunk_resolution: IntCoord,
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
