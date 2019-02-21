//! Tracking cells shared between neighboring chunks.

use super::ChunkOrigin;
use crate::grid::{Point3, PosInOwningRoot};

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
    pub sink: Point3,
}
