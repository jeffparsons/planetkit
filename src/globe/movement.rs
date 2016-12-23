use super::{ CellPos, Dir };
use super::cell_shape::NEIGHBOR_OFFSETS;

// TODO: take resolution, too.
// TODO: remark that it might modify the arguments
// even if the result is an error.
// TODO: remark on assuming a valid pos for this resolution.
pub fn move_forward(
    pos: &mut CellPos,
    dir: &mut Dir,
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

    // TODO: rebase on other root quads as necessary
    // both before and after stepping?

    // Nothing bad happened up to here; presumably we successfully
    // advanced by one step and rebased on whatever root quad we're
    // now pointing into if necessary.
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::{ CellPos, Dir };

    #[test]
    fn move_forward_in_positive_x_direction() {
        let mut pos = CellPos::default();
        let mut dir = Dir::default();
        move_forward(&mut pos, &mut dir).unwrap();
        assert_eq!(CellPos::default().set_x(1), pos);
        assert_eq!(Dir::default(), dir);
    }
}
