use super::{
    IntCoord,
    CellPos,
    Dir,
    pos_in_owning_root,
};
use movement::{
    move_forward,
    turn_left_by_one_hex_edge,
};

// TODO: this whole implementation is horribly inefficient,
// but it was the easiest one I could think of to get up and
// running quickly without having to handle a bunch of special
// cases. Replace me!

/// Iterator over the neighbors of a `Pos` at a given resolution.
pub struct Neighbors {
    resolution: [IntCoord; 2],
    origin: CellPos,
    first_neighbor: Option<CellPos>,
    current_dir: Dir,
}

impl Neighbors {
    pub fn new(mut pos: CellPos, resolution: [IntCoord; 2]) -> Neighbors {
        // Pick a direction that's valid for `pos`.
        // To do this, first express the position in its owning root...
        pos = pos_in_owning_root(pos, resolution);
        // ...after which we know that direction 0 is valid, except for
        // the south pole. Refer to the diagram in `pos_in_owning_root`
        // to see how this falls out.
        let start_dir = if pos.x == resolution[0] && pos.y == resolution[1] {
            Dir::new(6)
        } else {
            Dir::new(0)
        };
        Neighbors {
            resolution: resolution,
            origin: pos,
            first_neighbor: None,
            current_dir: start_dir,
        }
    }
}

impl Iterator for Neighbors {
    type Item = CellPos;

    fn next(&mut self) -> Option<CellPos> {
        // Find the neighbor in the current direction.
        let mut pos = self.origin;
        let mut dir = self.current_dir;
        move_forward(&mut pos, &mut dir, self.resolution).expect("Oops, we started from an invalid position.");

        // Express neighbor in its owning root so we know
        // whether we've seen it twice.
        pos = pos_in_owning_root(pos, self.resolution);

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
