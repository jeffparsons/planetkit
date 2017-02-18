use std::iter::Chain;
use std::slice;
use super::{
    IntCoord,
    CellPos,
    PosInOwningRoot,
    Dir,
};
use movement::{
    move_forward,
    turn_left_by_one_hex_edge,
};

pub struct Neighbors {
    // Hide the iterators used to implement this.
    iter: NeighborsImpl,
}

/// Iterator over the cells sharing an interface with a given `pos`.
/// Doesn't include diagonal neighbors, e.g., one across and one down.
impl Neighbors {
    pub fn new(pos: CellPos, resolution: [IntCoord; 2]) -> Neighbors {
        let above_and_below = AboveAndBelow::new(pos);
        let is_away_from_root_edges =
            pos.x > 0 &&
            pos.x < resolution[0] - 1 &&
            pos.y > 0 &&
            pos.y < resolution[1] - 1;
        if is_away_from_root_edges {
            let fast_intra_root_neighbors = FastIntraRootNeighbors::new(pos);
            Neighbors {
                iter: NeighborsImpl::FastIntra(above_and_below.chain(fast_intra_root_neighbors)),
            }
        } else {
            let slow_general_edge_neighbors = SlowGeneralEdgeNeighbors::new(pos, resolution);
            Neighbors {
                iter: NeighborsImpl::SlowGeneral(above_and_below.chain(slow_general_edge_neighbors)),
            }
        }
    }
}

impl Iterator for Neighbors {
    type Item = CellPos;

    fn next(&mut self) -> Option<CellPos> {
        match self.iter {
            NeighborsImpl::SlowGeneral(ref mut iter) => iter.next(),
            NeighborsImpl::FastIntra(ref mut iter) => iter.next(),
        }
    }
}

// There are a couple of different implementations; we pick the fast
// one if we can, or otherwise fall back to the general one.
enum NeighborsImpl {
    SlowGeneral(Chain<AboveAndBelow, SlowGeneralEdgeNeighbors>),
    FastIntra(Chain<AboveAndBelow, FastIntraRootNeighbors>),
}

// TODO: this whole implementation is horribly inefficient,
// but it was the easiest one I could think of to get up and
// running quickly without having to handle a bunch of special
// cases. Replace me!

/// Iterator over the cells sharing an interface with a given `pos`.
/// Doesn't include diagonal neighbors, e.g., one across and one down.
struct SlowGeneralEdgeNeighbors {
    resolution: [IntCoord; 2],
    origin: CellPos,
    first_neighbor: Option<CellPos>,
    current_dir: Dir,
}

impl SlowGeneralEdgeNeighbors {
    pub fn new(mut pos: CellPos, resolution: [IntCoord; 2]) -> SlowGeneralEdgeNeighbors {
        // Pick a direction that's valid for `pos`.
        // To do this, first express the position in its owning root...
        pos = PosInOwningRoot::new(pos, resolution).into();
        // ...after which we know that direction 0 is valid, except for
        // the south pole. Refer to the diagram in `PosInOwningRoot`
        // to see how this falls out.
        let start_dir = if pos.x == resolution[0] && pos.y == resolution[1] {
            Dir::new(6)
        } else {
            Dir::new(0)
        };
        SlowGeneralEdgeNeighbors {
            resolution: resolution,
            origin: pos,
            first_neighbor: None,
            current_dir: start_dir,
        }
    }
}

impl Iterator for SlowGeneralEdgeNeighbors {
    type Item = CellPos;

    fn next(&mut self) -> Option<CellPos> {
        // Find the neighbor in the current direction.
        let mut pos = self.origin;
        let mut dir = self.current_dir;
        move_forward(&mut pos, &mut dir, self.resolution).expect("Oops, we started from an invalid position.");

        // Express neighbor in its owning root so we know
        // whether we've seen it twice.
        pos = PosInOwningRoot::new(pos, self.resolution).into();

        if let Some(first_neighbor) = self.first_neighbor {
            if first_neighbor == pos {
                // We've already emitted this neighbor.
                return None;
            }
        } else {
            self.first_neighbor = Some(pos);
        }

        // Turn to face the next neighbor.
        turn_left_by_one_hex_edge(&mut self.origin, &mut self.current_dir, self.resolution).expect("Oops, we picked a bad starting direction.");

        // Yield the neighbor we found above.
        Some(pos)
    }
}

/// Iterator over the cells sharing an interface with a given `pos`.
/// Doesn't include diagonal neighbors, e.g., one across and one down.
///
/// Assumes that all neighbors are within the same chunk as the center cell.
/// Behaviour is undefined if this is not true.
struct FastIntraRootNeighbors {
    origin: CellPos,
    offsets: slice::Iter<'static, (IntCoord, IntCoord)>,
}

impl FastIntraRootNeighbors {
    pub fn new(pos: CellPos) -> FastIntraRootNeighbors {
        use super::cell_shape::NEIGHBOR_OFFSETS;
        FastIntraRootNeighbors {
            origin: pos,
            offsets: NEIGHBOR_OFFSETS.iter(),
        }
    }
}

impl Iterator for FastIntraRootNeighbors {
    type Item = CellPos;

    fn next(&mut self) -> Option<CellPos> {
        self.offsets.next().map(|offset| {
            self.origin
                .clone()
                .set_x(self.origin.x + offset.0)
                .set_y(self.origin.y + offset.1)
        })
    }
}

// Iterator over cell positions immediately above and below
// a given cell.
//
// Does not yield the (invalid) position below if the center cell is at `z == 0`.
struct AboveAndBelow {
    origin: CellPos,
    yielded_above: bool,
    yielded_below: bool,
}

impl AboveAndBelow {
    pub fn new(pos: CellPos) -> AboveAndBelow {
        AboveAndBelow {
            origin: pos,
            yielded_above: false,
            yielded_below: false,
        }
    }
}

impl Iterator for AboveAndBelow {
    type Item = CellPos;

    fn next(&mut self) -> Option<CellPos> {
        if !self.yielded_above {
            // Yield position above.
            self.yielded_above = true;
            return Some(self.origin.clone().set_z(self.origin.z + 1));
        }

        if !self.yielded_below {
            if self.origin.z == 0 {
                // There's no valid position below.
                return None;
            }

            // Yield position below.
            self.yielded_below = true;
            return Some(self.origin.clone().set_z(self.origin.z - 1));
        }

        None
    }
}
