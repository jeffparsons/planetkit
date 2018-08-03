use grid::{Dir, GridCoord, GridPoint3};

use super::transform::*;
use super::util::*;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum TurnDir {
    Left,
    Right,
}

impl TurnDir {
    pub fn opposite(&self) -> TurnDir {
        match *self {
            TurnDir::Left => TurnDir::Right,
            TurnDir::Right => TurnDir::Left,
        }
    }

    /// Turn one unit left or right.
    ///
    /// Note that this does not turn from one hexagon edge
    /// to another, but from an edge to a vertex, or vice versa.
    ///
    /// Note also that this can not rebase on another root quad;
    /// the direction faced will not necessarily be the canonical
    /// representation of that direction.
    pub fn apply_one_unit(&self, dir: &mut Dir) {
        match *self {
            TurnDir::Left => dir.index = (dir.index + 1) % 12,
            TurnDir::Right => dir.index = (dir.index + 12 - 1) % 12,
        };
    }

    /// Turn two units left or right.
    ///
    /// This turns from one hexagon edge to another, or one hexagon
    /// vertex to another.
    ///
    /// Note that this can not rebase on another root quad;
    /// the direction faced will not necessarily be the canonical
    /// representation of that direction.
    pub fn apply_two_units(&self, dir: &mut Dir) {
        match *self {
            TurnDir::Left => dir.index = (dir.index + 2) % 12,
            TurnDir::Right => dir.index = (dir.index + 12 - 2) % 12,
        };
    }
}

/// See `turn_by_one_hex_edge`.
pub fn turn_left_by_one_hex_edge(
    pos: &mut GridPoint3,
    dir: &mut Dir,
    resolution: [GridCoord; 2],
) -> Result<(), ()> {
    turn_by_one_hex_edge(pos, dir, resolution, TurnDir::Left)
}

/// See `turn_by_one_hex_edge`.
pub fn turn_right_by_one_hex_edge(
    pos: &mut GridPoint3,
    dir: &mut Dir,
    resolution: [GridCoord; 2],
) -> Result<(), ()> {
    turn_by_one_hex_edge(pos, dir, resolution, TurnDir::Right)
}

/// Returns an error if `pos` and `dir` do not point at an edge
/// of the current cell; this is intended for rotating to valid
/// directions for forward movement, which isnt possible if the starting
/// direction is illegal.
///
/// Behaviour is undefined if `pos` and `dir` are not already in their
/// canonical form; i.e. if `pos` is on the boundary of two root quads,
/// then `dir` points into that quad or along its edge, but not out.
///
/// This extends to behaviour being undefined if `pos` lies outside the
/// bounds of its root quad.
///
/// Both these cases will panic in debug mode.
pub fn turn_by_one_hex_edge(
    pos: &mut GridPoint3,
    dir: &mut Dir,
    resolution: [GridCoord; 2],
    turn_dir: TurnDir,
) -> Result<(), ()> {
    debug_assert_pos_within_root(pos, resolution);

    // Only allow turning from and to valid directions for forward movement.
    //
    // The special nature of the 12 pentagons is only relevant
    // when considering the interface between quads, so for this part we
    // can treat both cells as hexagons.
    if !dir.points_at_hex_edge() {
        return Err(());
    }

    #[cfg(debug)]
    {
        let next_pos = adjacent_pos_in_dir(*pos, *dir)?;
        // Pos should still be within the root bounds; otherwise it means
        // `pos` and `dir` were not in their canonical forms when passed
        // into this function. (`pos` should have been in a different root.)
        debug_assert_pos_within_root(next_pos, resolution);
    }

    // Turn left by one hexagon edge.
    turn_dir.apply_two_units(dir);

    // Rebase on whatever root quad we're now pointing into if necessary.
    maybe_rebase_on_adjacent_root_following_rotation(pos, dir, resolution);

    // Nothing bad happened up to here; presumably we successfully
    // turned by one hexagon edge and rebased on whatever root quad we're
    // now pointing into if necessary.
    Ok(())
}

/// Assumes `pos` is either exactly on the interface between
/// two root quads, or somewhere within a root.
///
/// Panics if `dir` does not point to a direction that would
/// represent an immediately adjacent cell _if `pos` were if in a hexagon_
/// (which is not necessarily so).
fn maybe_rebase_on_adjacent_root_following_rotation(
    pos: &mut GridPoint3,
    dir: &mut Dir,
    resolution: [GridCoord; 2],
) {
    // We only might need to re-base if we're on the boundary of two root quads.
    if !is_on_root_edge(pos, resolution) {
        return;
    }

    // See diagram in `triangles.rs` to help reason about these.
    let tri = if is_pentagon(pos, resolution) {
        // Pick the triangle whose middle axis is closest to dir.
        // This only applies if we're sitting and rotating on a pentagon,
        // because that's when it's ambiguous which triangle we should
        // choose otherwise.
        triangle_on_pos_with_closest_mid_axis(pos, dir, resolution)
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

    // If the next step would be into the same root, then we can just transform
    // straight back to world coordinates via the same triangle
    let still_in_same_quad = next_pos.x >= 0 && next_pos.y >= 0;
    if still_in_same_quad {
        transform_into_exit_triangle(pos, dir, resolution, &tri.exits[0]);
        return;
    }

    // Turning left (pointing more east) around north pole.
    if next_pos.x < 0 {
        pos.x = pos.y;
        pos.y = 0;
        *dir = dir.next_hex_edge_right();

        transform_into_exit_triangle(pos, dir, resolution, &tri.exits[1]);
        return;
    }

    // Turning right (pointing more west) around north pole.
    if next_pos.y < 0 {
        pos.y = pos.x;
        pos.x = 0;
        *dir = dir.next_hex_edge_left();

        transform_into_exit_triangle(pos, dir, resolution, &tri.exits[4]);
        return;
    }

    // Uh oh, we must have missed a case.
    panic!("Oops, we must have forgotten a rotation case. Sounds like we didn't test hard enough!")
}

// Panics if we're not facing a valid direction for movement.
pub fn turn_around_and_face_neighbor(
    pos: &mut GridPoint3,
    dir: &mut Dir,
    resolution: [GridCoord; 2],
    last_turn_bias: super::TurnDir,
) {
    if is_pentagon(pos, resolution) {
        turn_by_one_hex_edge(pos, dir, resolution, last_turn_bias).unwrap();
        turn_by_one_hex_edge(pos, dir, resolution, last_turn_bias).unwrap();
    } else {
        turn_left_by_one_hex_edge(pos, dir, resolution).unwrap();
        turn_left_by_one_hex_edge(pos, dir, resolution).unwrap();
        turn_left_by_one_hex_edge(pos, dir, resolution).unwrap();
    }
}
