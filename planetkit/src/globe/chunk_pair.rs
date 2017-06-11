//! Tracking cells shared between neighboring chunks.

use grid::{ GridPoint3, PosInOwningRoot };
use super::ChunkOrigin;

#[derive(Hash, PartialEq, Eq)]
pub struct ChunkPairOrigins {
    pub source: ChunkOrigin,
    pub sink: ChunkOrigin,
}

pub struct ChunkPair {
    pub point_pairs: Vec<PointPair>,
    pub last_upstream_edge_version_known_downstream: u64,
}

#[derive(Clone)]
pub struct PointPair {
    pub source: PosInOwningRoot,
    pub sink: GridPoint3,
}
