use na;

use super::super::{ IntCoord, CellPos, Dir };
use super::super::cell_shape::NEIGHBOR_OFFSETS;
use super::triangles::*;

// Transform `pos` and `dir` to be relative to the given triangle's apex.
// This is used so that most calculations can be done assuming we're in the
// arctic triangle of root 0 (see `triangles.rs`) to minimise the amount of
// special case logic.
//
// This is also then used to transform back to the original space when done.
pub fn transform_relative_to_triangle(
    pos: &mut CellPos,
    dir: &mut Dir,
    _resolution: [IntCoord; 2],
    tri: &Triangle,
) {
    // Use nalgebra for some local transformations.
    // We are ignoring z-axis completely because this kid of movement
    // is only in (x, y).
    type Pos2 = na::Point2<IntCoord>;
    type PosVec2 = na::Vector2<IntCoord>;
    type PosMat2 = na::Matrix2<IntCoord>;

    // TODO: compute actual triangle apex based on resolution.
    let apex = Pos2::new(0, 0);

    let x_dir = tri.x_dir;
    let y_dir = (x_dir + 2) % 12;
    let x_edge_index = (x_dir / 2) as usize;
    let y_edge_index = (y_dir / 2) as usize;
    let pos2 = Pos2::new(pos.x, pos.y);
    let pos_from_tri_apex = (pos2 - apex).to_point();
    // TODO: lol does this make sense? Needs moar matrix examples
    // to see if this is even remotely right.
    let transform = PosMat2::new(
        NEIGHBOR_OFFSETS[x_edge_index].0, NEIGHBOR_OFFSETS[x_edge_index].1,
        NEIGHBOR_OFFSETS[y_edge_index].0, NEIGHBOR_OFFSETS[y_edge_index].1,
    );
    let new_pos: Pos2 = transform * pos_from_tri_apex;

    // Stuff it all back into the arguments.
    pos.x = new_pos.x;
    pos.y = new_pos.y;
    *dir = Dir::new((dir.index + 12 - x_dir) % 12);
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::triangles::*;
    use super::super::super::{ CellPos, Dir };

    const RESOLUTION: [i64; 2] = [32, 64];

    #[test]
    fn north_pole_to_north_pole_facing_x_is_noop() {
        // Transform from north pole to north pole.
        let mut pos = CellPos::default();
        let mut dir = Dir::default();
        let tri = &TRIANGLES[0];
        transform_relative_to_triangle(&mut pos, &mut dir, RESOLUTION, tri);
        assert_eq!(CellPos::default(), pos);
        assert_eq!(Dir::default(), dir);
    }

    #[test]
    fn north_pole_to_north_pole_facing_north_is_noop() {
        // Transform from north pole to north pole,
        // starting a bit south of the pole and pointing up.
        // NOTE: this isn't a valid direction to move in.
        let mut pos = CellPos::default().set_x(1).set_y(1);
        let mut dir = Dir::new(7);
        let tri = &TRIANGLES[0];
        transform_relative_to_triangle(&mut pos, &mut dir, RESOLUTION, tri);
        assert_eq!(CellPos::default().set_x(1).set_y(1), pos);
        assert_eq!(Dir::new(7), dir);
    }

    // TODO: a handful of tests around northern tropics;
    // those are the ones that will show up fundamental oopsies
    // in the way all this works.
    //
    // - Transform to arctic
    // - Round-trip transform should be no-op.
}
