use super::{ IntCoord, CellPos, Dir };
use super::cell_shape::NEIGHBOR_OFFSETS;

// TODO: remark that it might modify the arguments
// even if the result is an error.
// TODO: remark on assuming a valid pos for this resolution.
pub fn move_forward(
    pos: &mut CellPos,
    dir: &mut Dir,
    resolution: [IntCoord; 2],
) -> Result<(), ()> {
    // TODO: panic if pos is invalid? Or just return different
    // errors?

    // TODO: rebase on other root quads as necessary
    // both before and after stepping? It makes the rest
    // of the logic here easier.

    // Only allow moving into immediately adjacent cells.
    //
    // When moving strictly within a root quad, there's nothing
    // meaningfully pentagonal about the corners. So because we already
    // rebased on the quad we're moving into we can now treat whatever
    // cell we're in as being hexagonal.
    if !dir.points_at_hex_edge() {
        return Err(());
    }

    // Find the (x, y) offset for `dir` and apply to `pos`.
    // Edge index is half the direction index, because direction 0
    // points at edge 0.
    let edge_index = (dir.index / 2) as usize;
    let (dx, dy) = NEIGHBOR_OFFSETS[edge_index];
    pos.x += dx;
    pos.y += dy;

    // Rebase on whatever root quad we're now pointing into if necessary.
    rebase_on_root_faced(pos, dir, resolution);

    // Nothing bad happened up to here; presumably we successfully
    // advanced by one step and rebased on whatever root quad we're
    // now pointing into if necessary.
    Ok(())
}

// TODO: remark on assuming a valid pos for this resolution.
fn rebase_on_root_faced(
    pos: &mut CellPos,
    dir: &mut Dir,
    _resolution: [IntCoord; 2],
) {
    // For now, just handle moving one chunk to the east
    // in northern triangle.
    // TODO: proper implementation
    if pos.x == 0 && dir.index == 6 {
        pos.root = pos.root.next_east();
        pos.x = pos.y;
        pos.y = 0;
        dir.index = 4;
    }

    // Ruh roh, no way of knowing if we have to do anything!
}

// TODO: `turn_left` and `turn_right` functions that are smart
// about pentagons.

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{ CellPos, Dir };

    #[test]
    fn move_forward_in_positive_x_direction() {
        let resolution: [i64; 2] = [32, 64];

        let mut pos = CellPos::default();
        let mut dir = Dir::default();
        move_forward(&mut pos, &mut dir, resolution).unwrap();
        assert_eq!(CellPos::default().set_x(1), pos);
        assert_eq!(Dir::default(), dir);
    }

    #[test]
    fn move_east_under_north_pole() {
        let resolution: [i64; 2] = [32, 64];

        // Start just south of the north pole in root 4,
        // facing north-east.
        let mut pos = CellPos::default()
            .set_root(4)
            .set_x(1)
            .set_y(1);
        let mut dir = Dir::new(6);
        move_forward(&mut pos, &mut dir, resolution).unwrap();

        // We should now be on the edge of root 4 and 0,
        // facing east into root 0.
        assert_eq!(CellPos::default().set_x(1), pos);
        assert_eq!(Dir::new(4), dir);
    }
}
