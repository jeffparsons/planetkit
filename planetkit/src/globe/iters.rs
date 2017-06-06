use grid::{ GridCoord, GridPoint3 };
use super::ChunkOrigin;

// TODO: rename this file; probably neater to have smaller files
// rather than having multiple non-trivial iterators together.

// TODO: none of the names in here really properly describe
// what they're referring to.

// Variants with `SameX` in their name are only applicable
// before the south-western (high-x) root boundary.
//
// Variants with `SameY` in their name are only applicable
// before the south-eastern (high-y) root boundary.
//
// Variants with `PrevX` in their name are only applicable
// after the north-eastern (low-x) root boundary, and only where
// the point lies on a chunk boundary in the x-direction.
//
// Variants with `PrevY` in their name are only applicable
// after the north-western (low-y) root boundary, and only where
// the point lies on a chunk boundary in the y-direction.
//
// I've left a diagram below of a root quad with 2x2 chunks to help
// visualise this.
//
//          ●
//         / \
//        / · \
//       / · · \
//      ● · · · ●
//     / \ · · / \
//    / · \ · / · \
//   / · · \ / · · \
//  ● · · · ● · · · ●
//   \ · · / \ · · /
//    \ · / · \ · /
//     \ / · · \ /
//      ● · · · ●
//       \ · · /
//        \ · /
//         \ /
//          ●
//
enum CandidateChunk {
    SameXSameY,
    SameXPrevY,
    PrevXSameY,
    PrevXPrevY,
    Done,
}

pub struct ChunksInSameRootContainingPoint {
    point: GridPoint3,
    root_resolution: [GridCoord; 2],
    chunk_resolution: [GridCoord; 3],
    next_candidate_chunk: CandidateChunk,
}

impl ChunksInSameRootContainingPoint {
    pub fn new(point: GridPoint3, root_resolution: [GridCoord; 2], chunk_resolution: [GridCoord; 3]) -> ChunksInSameRootContainingPoint {
        ChunksInSameRootContainingPoint {
            point: point,
            root_resolution: root_resolution,
            chunk_resolution: chunk_resolution,
            next_candidate_chunk: CandidateChunk::SameXSameY,
        }
    }

    fn has_same_x_chunk(&self) -> bool {
        self.point.x < self.root_resolution[0]
    }

    fn has_same_y_chunk(&self) -> bool {
        self.point.y < self.root_resolution[1]
    }

    fn has_prev_x_chunk(&self) -> bool {
        self.point.x > 0
        &&
        // On chunk boundary in x-direction
        self.point.x == self.same_chunk_x()
    }

    fn has_prev_y_chunk(&self) -> bool {
        self.point.y > 0
        &&
        // On chunk boundary in y-direction
        self.point.y == self.same_chunk_y()
    }

    fn same_chunk_x(&self) -> GridCoord {
        self.point.x / self.chunk_resolution[0] * self.chunk_resolution[0]
    }

    fn same_chunk_y(&self) -> GridCoord {
        self.point.y / self.chunk_resolution[1] * self.chunk_resolution[1]
    }

    fn prev_chunk_x(&self) -> GridCoord {
        (self.point.x / self.chunk_resolution[0] - 1) * self.chunk_resolution[0]
    }

    fn prev_chunk_y(&self) -> GridCoord {
        (self.point.y / self.chunk_resolution[1] - 1) * self.chunk_resolution[1]
    }

    fn chunk_origin(&self, chunk_x: GridCoord, chunk_y: GridCoord) -> Option<ChunkOrigin> {
        Some(ChunkOrigin::new(
            GridPoint3::new(
                self.point.root,
                chunk_x,
                chunk_y,
                self.point.z / self.chunk_resolution[2] * self.chunk_resolution[2],
            ),
            self.root_resolution,
            self.chunk_resolution,
        ))
    }
}

impl Iterator for ChunksInSameRootContainingPoint {
    type Item = ChunkOrigin;

    fn next(&mut self) -> Option<ChunkOrigin> {
        match self.next_candidate_chunk {
            CandidateChunk::SameXSameY => {
                self.next_candidate_chunk = CandidateChunk::SameXPrevY;
                if self.has_same_x_chunk() && self.has_same_y_chunk() {
                    self.chunk_origin(
                        self.same_chunk_x(),
                        self.same_chunk_y(),
                    )
                } else {
                    self.next()
                }
            },
            CandidateChunk::SameXPrevY => {
                self.next_candidate_chunk = CandidateChunk::PrevXSameY;
                if self.has_same_x_chunk() && self.has_prev_y_chunk() {
                    self.chunk_origin(
                        self.same_chunk_x(),
                        self.prev_chunk_y(),
                    )
                } else {
                    self.next()
                }
            },
            CandidateChunk::PrevXSameY => {
                self.next_candidate_chunk = CandidateChunk::PrevXPrevY;
                if self.has_prev_x_chunk() && self.has_same_y_chunk() {
                    self.chunk_origin(
                        self.prev_chunk_x(),
                        self.same_chunk_y(),
                    )
                } else {
                    self.next()
                }
            },
            CandidateChunk::PrevXPrevY => {
                self.next_candidate_chunk = CandidateChunk::Done;
                if self.has_prev_x_chunk() && self.has_prev_y_chunk() {
                    self.chunk_origin(
                        self.prev_chunk_x(),
                        self.prev_chunk_y(),
                    )
                } else {
                    self.next()
                }
            },
            CandidateChunk::Done => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_for_point_straddling_two_chunks() {
        // Pick an arbitrary semi-interesting point and make sure we find
        // exactly the chunk origins we should.
        //
        //      ●
        //     / \
        //    ◌ · ◌
        //   / \ / \
        //  ● · ◌ · ●
        //   \ / \ / \
        //    ◌ · ◌ · ◌
        //     \ / \ / \
        //      ● · ◌ · ●
        //       \ x \ /
        //        ◌ · ◌
        //         \ /
        //          ●
        const ROOT_RESOLUTION: [GridCoord; 2] = [4, 8];
        const CHUNK_RESOLUTION: [GridCoord; 3] = [2, 2, 64];
        let point = GridPoint3::new(
            // Arbitrary; just to make sure it flows throught to the chunk origin returned
            4.into(),
            3,
            6,
            // Kinda arbitrary; just to make sure it gets calculated based on resolution
            77,
        );
        let chunks_iter = ChunksInSameRootContainingPoint::new(point, ROOT_RESOLUTION, CHUNK_RESOLUTION);
        let chunk_origins: Vec<ChunkOrigin> = chunks_iter.collect();
        assert_eq!(chunk_origins.len(), 2);
        // This is the chunk just north-west of the point.
        assert!(chunk_origins.contains(
            &ChunkOrigin::new(
                GridPoint3::new(
                    // Root made it through
                    4.into(),
                    // This is the chunk just north-west of the point.
                    2,
                    4,
                    // Z-coordinate of chunk was calculated correctly
                    64,
                ),
                ROOT_RESOLUTION,
                CHUNK_RESOLUTION,
            )
        ));
        // This is the chunk just south-west of the point.
        assert!(chunk_origins.contains(
            &ChunkOrigin::new(
                GridPoint3::new(
                    // Root made it through
                    4.into(),
                    // This is the chunk just south-west of the point.
                    2,
                    6,
                    // Z-coordinate of chunk was calculated correctly
                    64,
                ),
                ROOT_RESOLUTION,
                CHUNK_RESOLUTION,
            )
        ));
    }

    #[test]
    fn number_of_chunks_for_all_kinds_of_points() {
        // Exhaustively test all equivalence classes of points in a chunk,
        // but only to the point of _how many_ chunks each of those points
        // should belong to.
        //
        // The single-digit number at each point below represents how many
        // chunks that point should straddle.
        //
        //      1
        //     1 1
        //    2 1 2
        //   1 2 2 1
        //  1 1 4 1 2
        //   1 2 2 2 1
        //    2 1 4 1 2
        //     1 2 2 2 1
        //      2 1 4 1 1
        //       1 2 2 1
        //        2 1 2
        //         1 1
        //          1
        const ROOT_RESOLUTION: [GridCoord; 2] = [4, 8];
        const CHUNK_RESOLUTION: [GridCoord; 3] = [2, 2, 64];
        let points = (0..9).flat_map(|y| {
            (0..5).map(move |x| {
                GridPoint3::new(
                    // Arbitrary
                    3.into(),
                    x,
                    y,
                    // Arbitrary
                    123456,
                )
            })
        });
        let chunk_counts: Vec<usize> = points.map(|point| {
            ChunksInSameRootContainingPoint::new(point, ROOT_RESOLUTION, CHUNK_RESOLUTION).count()
        }).collect();
        assert_eq!(chunk_counts, vec![
            1, 1, 2, 1, 1,
            1, 1, 2, 1, 1,
            2, 2, 4, 2, 2,
            1, 1, 2, 1, 1,
            2, 2, 4, 2, 2,
            1, 1, 2, 1, 1,
            2, 2, 4, 2, 2,
            1, 1, 2, 1, 1,
            1, 1, 2, 1, 1,
        ]);
    }
}
