use crate::na;

use crate::cell_shape::NEIGHBOR_OFFSETS;
use crate::{Dir, GridCoord, GridPoint3};

use super::transform::*;
use super::triangles::*;

/// Get next cell in direction faced by `dir`, without considering
/// movement between roots. Note that this may therefore return positions
/// outside the boundaries of `pos`'s current root.
///
/// Returns an error if `dir` does not point to a direction that would
/// represent an immediately adjacent cell if in a hexagon. (Movement
/// toward vertices is undefined.)
pub fn adjacent_pos_in_dir(pos: GridPoint3, dir: Dir) -> Result<GridPoint3, ()> {
    if !dir.points_at_hex_edge() {
        return Err(());
    }

    // Find the (x, y) offset for `dir` and apply to `pos`.
    // Edge index is half the direction index, because direction 0
    // points at edge 0.
    let edge_index = (dir.index / 2) as usize;
    let (dx, dy) = NEIGHBOR_OFFSETS[edge_index];
    Ok(GridPoint3::new(pos.root, pos.x + dx, pos.y + dy, pos.z))
}

// Transform (x, y, dir) back to local coordinates near where we started,
// taking account any change in root required.
pub fn transform_into_exit_triangle(
    pos: &mut GridPoint3,
    dir: &mut Dir,
    resolution: [GridCoord; 2],
    exit: &Exit,
) {
    let exit_tri = &TRIANGLES[exit.triangle_index];
    pos.root.index = (pos.root.index + exit.root_offset) % 5;
    let (new_pos, new_dir) = local_to_world(*pos, *dir, resolution, exit_tri);
    *pos = new_pos;
    *dir = new_dir;
}

/// Pick triangle with the closest apex that is oriented such that `pos` lies
/// between its x-axis and y-axis.
///
/// If `pos` is on a pentagon, you probably won't want this.
/// Consider `triangle_on_pos_with_closest_mid_axis` instead?
pub fn closest_triangle_to_point(
    pos: &GridPoint3,
    resolution: [GridCoord; 2],
) -> &'static Triangle {
    // First we filter down to those where
    // pos lies between the triangle's x-axis and y-axis.
    // (See diagram in `triangles.rs`.)
    //
    // It is important that we don't pick a differently-oriented
    // triangle with the same apex, because that would sometimes
    // lead us to unnecessarily transforming positions into
    // neighboring quads. (We try to maintain stability within a given
    // quad in general, and there's a bunch of logic around here in particular
    // that assumes that.)
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
    type Pos2 = na::Point2<GridCoord>;
    let pos2 = Pos2::new(pos.x, pos.y);
    candidate_triangles
        .iter()
        .min_by_key(|triangle| {
            // Both parts of the apex are expressed in terms of x-dimension.
            let apex = Pos2::new(triangle.apex[0], triangle.apex[1]) * resolution[0];
            let apex_to_pos = (pos2 - apex).abs();
            // Hex distance from apex to pos
            apex_to_pos.x + apex_to_pos.y
        })
        .expect("There should have been exactly three items; this shouldn't be possible!")
}

/// For whatever 1-3 triangles `pos` is sitting atop, find the one
/// whose "middle axis" (half-way between x-axis and y-axis) is closest
/// to `dir`.
///
/// This is useful for re-basing while turning, without unnecessarily
/// re-basing into a neighbouring root.
///
/// Panics called with any pos that is not a pentagon.
pub fn triangle_on_pos_with_closest_mid_axis(
    pos: &GridPoint3,
    dir: &Dir,
    resolution: [GridCoord; 2],
) -> &'static Triangle {
    // If `pos` sits on a pentagon and we're re-basing, then that probably
    // means we're turning. Because we're on a pentagon, it's important that
    // we select the triangle that is most closely oriented to our direction,
    // so that we don't accidentally re-base into a neighbouring quad unnecessarily.
    // (We try to maintain stability within a given quad in general, and there's a
    // bunch of logic around here in particular that assumes that.)
    type Pos2 = na::Point2<GridCoord>;
    let pos2 = Pos2::new(pos.x, pos.y);
    TRIANGLES
        .iter()
        .filter(|triangle| {
            // There will be between one and three triangles that
            // we are exactly on top of.
            use num_traits::Zero;
            // Both parts of the apex are expressed in terms of x-dimension.
            let apex = Pos2::new(triangle.apex[0], triangle.apex[1]) * resolution[0];
            let apex_to_pos = (pos2 - apex).abs();
            apex_to_pos.is_zero()
        })
        .min_by_key(|triangle| {
            // Find triangle with minimum angle between its "mid axis"
            // and wherever `pos` is pointing.
            let middle_axis_dir: i16 = (triangle.x_dir as i16 + 1) % 12;
            let mut a = middle_axis_dir - dir.index as i16;
            if a > 6 {
                a -= 12;
            } else if a < -6 {
                a += 12;
            }
            a.abs()
        })
        .expect("There should have been 1-3 triangles; did you call this with a non-pentagon pos?")
}

pub fn is_pentagon(pos: &GridPoint3, resolution: [GridCoord; 2]) -> bool {
    // There are six pentagons in every root quad:
    //
    //              ◌ north
    //             / \
    //            /   \
    //      west ◌     ◌ north-east
    //            \     \
    //             \     \
    //   south-west ◌     ◌ east
    //               \   /
    //                \ /
    //           south ◌
    //
    let is_north = pos.x == 0 && pos.y == 0;
    let is_north_east = pos.x == 0 && pos.y == resolution[0];
    let is_east = pos.x == 0 && pos.y == resolution[1];
    let is_west = pos.x == resolution[0] && pos.y == 0;
    let is_south_west = pos.x == resolution[0] && pos.y == resolution[0];
    let is_south = pos.x == resolution[0] && pos.y == resolution[1];
    is_north || is_north_east || is_east || is_west || is_south_west || is_south
}

pub fn is_on_root_edge(pos: &GridPoint3, resolution: [GridCoord; 2]) -> bool {
    pos.x == 0 || pos.y == 0 || pos.x == resolution[0] || pos.y == resolution[1]
}

pub fn debug_assert_pos_within_root(pos: &mut GridPoint3, resolution: [GridCoord; 2]) {
    debug_assert!(
        pos.x >= 0 && pos.y >= 0 && pos.x <= resolution[0] && pos.y <= resolution[1],
        "`pos` was outside its root at the given resolution."
    );
}
