use itertools;
use std::ops;

use super::ChunkOrigin;
use crate::grid::{GridCoord, Point3, Root};

/// Iterate over all the points in a chunk that are shared with any
/// other chunk. That is, those on the planes of x=0, x=max, y=0, and y=max,
/// but neither the top nor bottom planes.
pub struct ChunkSharedPoints {
    // TODO: optimise this to never even consider the internal grid points;
    // this is quite easy to write out as a product of chains or iterators,
    // but they have maps (with closures) in the middle, so I'm not sure
    // how to write their type out so I can store it.
    root: Root,
    x_min: GridCoord,
    x_max: GridCoord,
    y_min: GridCoord,
    y_max: GridCoord,
    iter: itertools::ConsTuples<
        itertools::Product<
            itertools::Product<ops::RangeInclusive<GridCoord>, ops::RangeInclusive<GridCoord>>,
            ops::Range<GridCoord>,
        >,
        ((GridCoord, GridCoord), GridCoord),
    >,
}

impl ChunkSharedPoints {
    pub fn new(chunk_origin: ChunkOrigin, chunk_resolution: [GridCoord; 3]) -> ChunkSharedPoints {
        let pos = chunk_origin.pos();
        let iter = iproduct!(
            // Include the far edge.
            pos.x..=(pos.x + chunk_resolution[0]),
            pos.y..=(pos.y + chunk_resolution[1]),
            // Chunks don't share points in the z-direction,
            // but do in the x- and y-directions.
            pos.z..(pos.z + chunk_resolution[2])
        );
        ChunkSharedPoints {
            root: pos.root,
            x_min: pos.x,
            x_max: pos.x + chunk_resolution[0],
            y_min: pos.y,
            y_max: pos.y + chunk_resolution[1],
            iter,
        }
    }
}

impl Iterator for ChunkSharedPoints {
    type Item = Point3;

    fn next(&mut self) -> Option<Point3> {
        if let Some(xyz) = self.iter.next() {
            let (x, y, z) = xyz;
            // Only return points that are on x=0, y=0, x=max, or y=max.
            let is_x_lim = x == self.x_min || x == self.x_max;
            let is_y_lim = y == self.y_min || y == self.y_max;
            // TODO: use `is_point_shared` instead.
            //
            // TODO: Rewrite this whole file to use a pre-computed
            // list for a given chunk size. (Or just cached on the Globe).
            // Because this whole thing is just terribly slow.
            let is_shared_point = is_x_lim || is_y_lim;
            if is_shared_point {
                // It's an x-edge or y-edge point.
                Some(Point3::new(self.root, x, y, z))
            } else {
                // Skip it.
                self.next()
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn chunk_shared_points() {
        const ROOT_RESOLUTION: [GridCoord; 2] = [16, 32];
        const CHUNK_RESOLUTION: [GridCoord; 3] = [8, 8, 64];
        let chunk_origin = ChunkOrigin::new(
            Point3::new(
                // Arbitrary; just to make sure it flows throught to the chunk origin returned
                4.into(),
                8,
                8,
                192,
            ),
            ROOT_RESOLUTION,
            CHUNK_RESOLUTION,
        );
        let shared_points_iter = ChunkSharedPoints::new(chunk_origin, CHUNK_RESOLUTION);
        let shared_points: Vec<Point3> = shared_points_iter.collect();
        // Should have as many points as the whole chunk minus the column down
        // the middle of non-shared cells.
        assert_eq!(shared_points.len(), 9 * 9 * 64 - 7 * 7 * 64);
        // TODO: better assertions?
    }

    #[test]
    fn all_shared_points_are_in_same_chunk() {
        use crate::globe::origin_of_chunk_in_same_root_containing;

        const ROOT_RESOLUTION: [GridCoord; 2] = [16, 32];
        const CHUNK_RESOLUTION: [GridCoord; 3] = [8, 8, 64];
        let chunk_origin = ChunkOrigin::new(
            Point3::new(
                // Arbitrary; just to make sure it flows throught to the chunk origin returned
                4.into(),
                8,
                8,
                192,
            ),
            ROOT_RESOLUTION,
            CHUNK_RESOLUTION,
        );
        let origins_of_shared_points_iter = ChunkSharedPoints::new(chunk_origin, CHUNK_RESOLUTION)
            .map(|point| {
                origin_of_chunk_in_same_root_containing(point, ROOT_RESOLUTION, CHUNK_RESOLUTION)
            });
        let origins: HashSet<ChunkOrigin> = origins_of_shared_points_iter.collect();
        for origin in &origins {
            assert_eq!(origin.pos().root.index, 4);
            assert!(origin.pos().x >= chunk_origin.pos().x);
            assert!(origin.pos().x <= chunk_origin.pos().x + CHUNK_RESOLUTION[0]);
            assert!(origin.pos().y >= chunk_origin.pos().y);
            assert!(origin.pos().y <= chunk_origin.pos().y + CHUNK_RESOLUTION[1]);
            assert!(origin.pos().z >= chunk_origin.pos().z);
            assert!(origin.pos().z < chunk_origin.pos().z + CHUNK_RESOLUTION[2]);
        }
    }
}
