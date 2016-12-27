mod triangles;
mod transform;
#[cfg(test)]
mod test;

use na;

use super::{ IntCoord, CellPos, Dir };
use super::cell_shape::NEIGHBOR_OFFSETS;

use self::triangles::*;
use self::transform::*;

// See `triangles.rs` for discussion about the approach used here,
// and the list of all triangles used.

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
    // Or maybe just assert that it's already done?
    // Ensuring this is true could then be a responsibility
    // of the caller... which will mostly be CellDweller?

    // Only allow moving into immediately adjacent cells.
    //
    // We've already established at this point that we will be moving
    // to a cell that is within the same root quad as the one we are
    // already in. The special nature of the 12 pentagons is only relevant
    // when considering the interface between quads, so for this part we
    // can treat both cells as hexagons.
    if !dir.points_at_hex_edge() {
        return Err(());
    }

    *pos = adjacent_pos_in_dir(*pos, *dir)?;

    // Rebase on whatever root quad we're now pointing into if necessary.
    maybe_rebase_on_adjacent_root(pos, dir, resolution);

    // Nothing bad happened up to here; presumably we successfully
    // advanced by one step and rebased on whatever root quad we're
    // now pointing into if necessary.
    Ok(())
}

/// Get next cell in direction faced by `dir`, without considering
/// movement between roots. Note that this may therefore return positions
/// outside the boundaries of `pos`'s current root.
///
/// Returns an error if `dir` does not point to a direction that would
/// represent an immediately adjacent cell if in a hexagon. (Movement
/// toward vertices is undefined.)
fn adjacent_pos_in_dir(
    pos: CellPos,
    dir: Dir,
) -> Result<CellPos, ()> {
    if !dir.points_at_hex_edge() {
        return Err(());
    }

    // Find the (x, y) offset for `dir` and apply to `pos`.
    // Edge index is half the direction index, because direction 0
    // points at edge 0.
    let edge_index = (dir.index / 2) as usize;
    let (dx, dy) = NEIGHBOR_OFFSETS[edge_index];
    Ok(CellPos {
        root: pos.root,
        x: pos.x + dx,
        y: pos.y + dy,
        z: pos.z,
    })
}

/// Assumes `pos` is either exactly on the interface between
/// two root quads, or somewhere within a root.
///
/// Panics if `dir` does not point to a direction that would
/// represent an immediately adjacent cell _if `pos` were if in a hexagon_
/// (which is not necessarily so).
fn maybe_rebase_on_adjacent_root(
    pos: &mut CellPos,
    dir: &mut Dir,
    resolution: [IntCoord; 2],
) {
    // First check if `pos` clearly doesn't need to be re-based on a neighboring
    // root quad; i.e. it's not on the edge.
    //
    // Prefer this to falling through to this case so that we can avoid all the
    // computation below, and also detect movement cases we've missed and panic.
    // (See bottom of function.)
    let away_from_root_edges =
        pos.x > 0 &&
        pos.y > 0 &&
        pos.x < resolution[0] &&
        pos.y < resolution[1];
    if away_from_root_edges {
        return;
    }

    // Pick the closest triangle that is oriented such that `pos` lies
    // between its x-axis and y-axis.
    let tri = closest_triangle_to_point(pos, resolution);

    // Transform `pos` and `dir` to be relative to triangle apex;
    // i.e. so we can treat them as if they're in arctic space,
    // and then transform them back when we're done.
    let (new_pos, new_dir) = world_to_local(*pos, *dir, resolution, tri);
    *pos = new_pos;
    *dir = new_dir;

    let next_pos = adjacent_pos_in_dir(
        pos.clone(), dir.clone()
    ).expect("Caller should have assured we're pointing at a hex edge.");

    // Next check if `pos` doesn't need to be re-based on a neighboring root quad
    // because it's `next_pos` is still in this root. Note that we're not checking
    // the far edges because re-orienting `pos` and `next_pos` on arctic-equivalent
    // triangles like we're doing guarantees they'll never be near the far end.
    //
    // Prefer this to falling through to this case so that we can avoid all the
    // computation below, and also detect movement cases we've missed and panic.
    // (See bottom of function.)
    let still_in_same_quad =
        next_pos.x >= 0 &&
        next_pos.y >= 0;
    if still_in_same_quad {
        // Transform (x, y, dir) back to where we started.
        let (new_pos, new_dir) = local_to_world(*pos, *dir, resolution, tri);
        *pos = new_pos;
        *dir = new_dir;

        return;
    }

    // Moving east around north pole.
    if next_pos.x < 0 {
        pos.x = pos.y;
        pos.y = 0;
        *dir = dir.next_hex_edge_right();

        // Transform (x, y, dir) back to where we started,
        // taking account any change in root required.
        let exit = &tri.exits[0];
        let exit_tri = &TRIANGLES[exit.triangle_index];
        pos.root.index = (pos.root.index + exit.root_offset) % 5;
        let (new_pos, new_dir) = local_to_world(*pos, *dir, resolution, exit_tri);
        *pos = new_pos;
        *dir = new_dir;

        return;
    }

    // Moving west around north pole.
    if next_pos.y < 0 {
        pos.y = pos.x;
        pos.x = 0;
        *dir = dir.next_hex_edge_left();

        // Transform (x, y, dir) back to where we started,
        // taking account any change in root required.
        let exit = &tri.exits[3];
        let exit_tri = &TRIANGLES[exit.triangle_index];
        pos.root.index = (pos.root.index + exit.root_offset) % 5;
        let (new_pos, new_dir) = local_to_world(*pos, *dir, resolution, exit_tri);
        *pos = new_pos;
        *dir = new_dir;

        return;
    }

    // Uh oh, we must have missed a case.
    panic!("Oops, we must have forgotten a movement case. Sounds like we didn't test hard enough!")
}

// Pick the closest triangle that is oriented such that `pos` lies
// between its x-axis and y-axis.
fn closest_triangle_to_point(
    pos: &mut CellPos,
    resolution: [IntCoord; 2],
) -> &'static Triangle {
    // First we filter down to those where
    // pos lies between the triangle's x-axis and y-axis.
    let candidate_triangles = if pos.x + pos.y < resolution[0] {
        &TRIANGLES[0..3]
    } else if pos.y < resolution[0] {
        &TRIANGLES[3..6]
    } else if pos.x + pos.y < resolution[1] {
        &TRIANGLES[6..9]
    } else {
        &TRIANGLES[9..12]
    };

    // Pick the closest triangle.
    type Pos2 = na::Point2<IntCoord>;
    let pos2 = Pos2::new(pos.x, pos.y);
    candidate_triangles.iter().min_by_key(|triangle| {
        let apex = Pos2::new(
            // Both parts of the apex are expressed in terms of x-dimension.
            triangle.apex.x * resolution[0],
            triangle.apex.y * resolution[0],
        );
        let apex_to_pos = na::Absolute::abs(&(pos2 - apex));
        let hex_distance_from_apex_to_pos = apex_to_pos.x + apex_to_pos.y;
        hex_distance_from_apex_to_pos
    }).expect("There should have been exactly three items; this shouldn't be possible!")
}

// TODO: `turn_left` and `turn_right` functions that are smart
// about pentagons.
