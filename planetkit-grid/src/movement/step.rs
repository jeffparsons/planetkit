use crate::{Dir, GridCoord, Point3};

use super::transform::*;
use super::turn::turn_around_and_face_neighbor;
use super::util::*;

// TODO: rename these methods so that there's
// no method with an obvious name like "move_forward"
// that doesn't actually do what you want most of the time.

/// Move forward by one cell and ensure `dir` now points to a
/// legal direction for continued movement.
///
/// Uses `last_turn_bias` to decide which direction to turn if we,
/// land on a pentagon, and updates it to be its opposite. In this
/// way we balance out the direction turned over time. This is not
/// necessary for anything to work (we could always choose to turn
/// left) but it's aesthetically nice.
///
/// See `move_forward` for more details`.
pub fn step_forward_and_face_neighbor(
    pos: &mut Point3,
    dir: &mut Dir,
    resolution: [GridCoord; 2],
    last_turn_bias: &mut super::TurnDir,
) -> Result<(), ()> {
    move_forward(pos, dir, resolution)?;

    // If we're on a pentagon, we'll need to update which direction
    // we're facing to make it legal for another step.
    if is_pentagon(pos, resolution) {
        // We've already been re-based such that `pos` and `dir` are in
        // their canonical form, now. This means that we're facing into
        // a quad, and we have at least one legal direction both left and
        // right that we can turn to while still maintaining the canonical
        // representation. Pick the opposite of whichever we did last time.
        // We'll then turn the other way next time we're forced to decide.
        *last_turn_bias = last_turn_bias.opposite();
        last_turn_bias.apply_one_unit(dir);
    }

    // Nothing bad happened up to here; presumably we successfully
    // advanced by one step and rebased on whatever root quad we're
    // now pointing into if necessary.
    Ok(())
}

/// Move backward by one cell and ensure `dir` now points to a
/// legal direction for continued movement.
///
/// This by design un-does any movement and rotation performed
/// by stepping forwards.
///
/// See `step_forward_and_face_neighbor` for more details`.
pub fn step_backward_and_face_neighbor(
    pos: &mut Point3,
    dir: &mut Dir,
    resolution: [GridCoord; 2],
    last_turn_bias: &mut super::TurnDir,
) -> Result<(), ()> {
    // Turn around.
    turn_around_and_face_neighbor(pos, dir, resolution, *last_turn_bias);
    if is_pentagon(pos, resolution) {
        // Update turn bias; if we walk forward again, we want a _repeat_
        // of the movement we just un-did.
        *last_turn_bias = last_turn_bias.opposite();
    }

    // Step forward.
    step_forward_and_face_neighbor(pos, dir, resolution, last_turn_bias)?;

    // Turn back around.
    turn_around_and_face_neighbor(pos, dir, resolution, *last_turn_bias);
    if is_pentagon(pos, resolution) {
        // Update turn bias; if we walk forward again, we want a _repeat_
        // of the movement we just un-did.
        *last_turn_bias = last_turn_bias.opposite();
    }

    Ok(())
}

/// Returns an error if `pos` and `dir` do not point at an edge
/// of the current cell; it's illegal to move toward a cell vertex.
///
/// Behaviour is undefined if `pos` and `dir` are not already in their
/// canonical form; i.e. if `pos` is on the boundary of two root quads,
/// then `dir` points into that quad or along its edge, but not out.
///
/// This extends to behaviour being undefined if `pos` lies outside the
/// bounds of its root quad.
///
/// Both these cases will panic in debug mode.
//
// TODO: rename to make it obvious this is usually not what you want.
pub fn move_forward(pos: &mut Point3, dir: &mut Dir, resolution: [GridCoord; 2]) -> Result<(), ()> {
    debug_assert_pos_within_root(pos, resolution);

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

    // Pos should still be within the root bounds; otherwise it means
    // `pos` and `dir` were not in their canonical forms when passed
    // into this function. (`pos` should have been in a different root.)
    debug_assert_pos_within_root(pos, resolution);

    // Rebase on whatever root quad we're now pointing into if necessary.
    maybe_rebase_on_adjacent_root_following_movement(pos, dir, resolution);

    // Nothing bad happened up to here; presumably we successfully
    // advanced by one step and rebased on whatever root quad we're
    // now pointing into if necessary.
    Ok(())
}

/// Assumes `pos` is either exactly on the interface between
/// two root quads, or somewhere within a root.
///
/// Panics if `dir` does not point to a direction that would
/// represent an immediately adjacent cell _if `pos` were if in a hexagon_
/// (which is not necessarily so).
fn maybe_rebase_on_adjacent_root_following_movement(
    pos: &mut Point3,
    dir: &mut Dir,
    resolution: [GridCoord; 2],
) {
    // We only might need to re-base if we're on the boundary of two root quads.
    if !is_on_root_edge(pos, resolution) {
        return;
    }

    let tri = if is_pentagon(pos, resolution) {
        // TODO: handle pentagons. This might be as easy as recognising
        // this case, taking the exact opposite orientation, and then
        // using `triangle_on_pos_with_closest_mid_axis` to identify the
        // triangle we want to work with. Here's the code from handling
        // _rotation_ around a pentagon. You may just be able to add two
        // more special cases below... :)
        //
        // Pick the triangle whose middle axis is closest to dir.
        let dir_we_came_from = dir.opposite();
        triangle_on_pos_with_closest_mid_axis(pos, dir_we_came_from, resolution)
    } else {
        // Pick the closest triangle that is oriented such that `pos` lies
        // between its x-axis and y-axis.
        closest_triangle_to_point(pos, resolution)
    };

    // Transform `pos` and `dir` to be relative to triangle apex;
    // i.e. so we can treat them as if they're in arctic space,
    // and then transform them back when we're done.
    let (new_pos, new_dir) = world_to_local(*pos, *dir, resolution, tri);
    *pos = new_pos;
    *dir = new_dir;

    let next_pos = adjacent_pos_in_dir(*pos, *dir)
        .expect("Caller should have assured we're pointing at a hex edge.");

    // Next check if `pos` doesn't need to be re-based on a neighboring root quad
    // because it's `next_pos` is still in this root. Note that we're not checking
    // the far edges because re-orienting `pos` and `next_pos` on arctic-equivalent
    // triangles like we're doing guarantees they'll never be near the far end.
    //
    // Prefer this to falling through to this case so that we can avoid all the
    // computation below, and also detect movement cases we've missed and panic.
    // (See bottom of function.)
    let still_in_same_quad = next_pos.x >= 0 && next_pos.y >= 0;
    if still_in_same_quad {
        transform_into_exit_triangle(pos, dir, resolution, &tri.exits[0]);
        return;
    }

    // Moving north-east through north pole.
    if pos.x == 0 && pos.y == 0 && dir.index == 6 {
        *dir = Dir::new(1);
        transform_into_exit_triangle(pos, dir, resolution, &tri.exits[2]);
        return;
    }

    // Moving north-west through north pole.
    if pos.x == 0 && pos.y == 0 && dir.index == 8 {
        *dir = Dir::new(1);
        transform_into_exit_triangle(pos, dir, resolution, &tri.exits[3]);
        return;
    }

    // Moving east around north pole.
    if next_pos.x < 0 {
        pos.x = pos.y;
        pos.y = 0;
        *dir = dir.next_hex_edge_right();

        transform_into_exit_triangle(pos, dir, resolution, &tri.exits[1]);
        return;
    }

    // Moving west around north pole.
    if next_pos.y < 0 {
        pos.y = pos.x;
        pos.x = 0;
        *dir = dir.next_hex_edge_left();

        transform_into_exit_triangle(pos, dir, resolution, &tri.exits[4]);
        return;
    }

    // Uh oh, we must have missed a case.
    panic!("Oops, we must have forgotten a movement case. Sounds like we didn't test hard enough!")
}
