/// These functions are used so that most movement calculations can assume we're in the
/// arctic triangle of root 0 (see `triangles.rs`) to minimise the amount of
/// special case logic.

use na;

use globe::{ IntCoord, CellPos, Dir };
use globe::cell_shape::NEIGHBOR_OFFSETS;
use super::triangles::*;

// Use nalgebra for some local transformations.
// We are ignoring z-axis completely because this kid of movement
// is only in (x, y).
type Pos2 = na::Point2<IntCoord>;
type PosMat2 = na::Matrix2<IntCoord>;

/// Transform `pos` and `dir` as specified relative to a given triangles apex,
/// to be relative to the world, or equivalently to triangle 0 at the north pole.
pub fn local_to_world(
    pos: CellPos,
    dir: Dir,
    resolution: [IntCoord; 2],
    tri: &Triangle,
) -> (CellPos, Dir) {
    // Compute rotation `dir` relative to world.
    let x_dir = tri.x_dir;
    let y_dir = (x_dir + 2) % 12;
    let x_edge_index = (x_dir / 2) as usize;
    let y_edge_index = (y_dir / 2) as usize;
    let transform_to_world = PosMat2::new(
        NEIGHBOR_OFFSETS[x_edge_index].0, NEIGHBOR_OFFSETS[y_edge_index].0,
        NEIGHBOR_OFFSETS[x_edge_index].1, NEIGHBOR_OFFSETS[y_edge_index].1,
    );

    // Apply transform.
    let pos2 = Pos2::new(pos.x, pos.y);
    let mut new_pos2: Pos2 = transform_to_world * pos2;
    let new_dir = Dir::new((dir.index + x_dir) % 12);

    // Translate `pos` from being relative to `apex`, to being
    // relative to the world, ignoring orientation.
    //
    // Both parts of the apex are expressed in terms of x-dimension.
    let apex = tri.apex * resolution[0];
    new_pos2 += apex.to_vector();
    let mut new_pos = pos;
    new_pos.x = new_pos2.x;
    new_pos.y = new_pos2.y;

    (new_pos, new_dir)
}

/// Transform `pos` and `dir` to be relative to the given triangle's apex.
pub fn world_to_local(
    pos: CellPos,
    dir: Dir,
    resolution: [IntCoord; 2],
    tri: &Triangle,
) -> (CellPos, Dir) {
    // Both parts of the apex are expressed in terms of x-dimension.
    let apex = tri.apex * resolution[0];

    // Translate `pos` relative to `apex` ignoring orientation.
    let pos2 = Pos2::new(pos.x, pos.y);
    let pos_from_tri_apex = (pos2 - apex).to_point();

    // Compute rotation required to express `pos` and `dir` relative to apex.
    let x_dir = tri.x_dir;
    let y_dir = (x_dir + 2) % 12;
    let x_edge_index = (x_dir / 2) as usize;
    let y_edge_index = (y_dir / 2) as usize;
    // Nalgebra's inverse is cautious (error checking) and is currently implemented
    // in a way that precludes inverting matrices of integers.
    // Fortunately this made me realise the determinant of our axis pairs is
    // always equal to 1, so we can save ourselves a bit of calculation here.
    let transform_to_local = PosMat2::new(
        NEIGHBOR_OFFSETS[y_edge_index].1, -NEIGHBOR_OFFSETS[y_edge_index].0,
        -NEIGHBOR_OFFSETS[x_edge_index].1, NEIGHBOR_OFFSETS[x_edge_index].0,
    );

    // Apply transform.
    let new_pos2: Pos2 = transform_to_local * pos_from_tri_apex;
    let mut new_pos = pos;
    new_pos.x = new_pos2.x;
    new_pos.y = new_pos2.y;
    let new_dir = Dir::new((dir.index + 12 - x_dir) % 12);

    (new_pos, new_dir)
}

#[cfg(test)]
mod test {
    use super::*;
    use ::globe::{ CellPos, Dir };

    const RESOLUTION: [i64; 2] = [32, 64];

    #[test]
    fn world_to_tri_0_facing_x_is_noop() {
        // Transform from north pole to north pole.
        let pos = CellPos::default();
        let dir = Dir::default();
        let tri = &TRIANGLES[0];
        let (new_pos, new_dir) = world_to_local(pos, dir, RESOLUTION, tri);
        // Should be no-op.
        assert_eq!(pos, new_pos);
        assert_eq!(dir, new_dir);
    }

    #[test]
    fn world_to_tri_0_facing_north_is_noop() {
        // Transform from north pole to north pole,
        // starting a bit south of the pole and pointing up.
        // NOTE: this isn't a valid direction to move in,
        // but that doesn't matter; it's still valid to transform.
        let pos = CellPos::default().set_x(1).set_y(1);
        let dir = Dir::new(7);
        let tri = &TRIANGLES[0];
        let (new_pos, new_dir) = world_to_local(pos, dir, RESOLUTION, tri);
        // Should be no-op.
        assert_eq!(pos, new_pos);
        assert_eq!(dir, new_dir);
    }

    #[test]
    fn world_to_tri_4() {
        // Transform from just below northern tropic, facing north-west.
        let pos = CellPos::default()
            .set_x(2)
            .set_y(RESOLUTION[1] / 2 - 1);
        let dir = Dir::new(8);
        let tri = &TRIANGLES[4];
        let (new_pos, new_dir) = world_to_local(pos, dir, RESOLUTION, tri);
        // Should now be just below north pole, facing west.
        assert_eq!(
            CellPos::default()
                .set_x(1)
                .set_y(1),
            new_pos
        );
        assert_eq!(Dir::new(10), new_dir);
    }

    #[test]
    fn tri_4_to_world() {
        // Transform from just below north pole, facing west.

        let pos = CellPos::default()
                .set_x(1)
                .set_y(1);
        let dir = Dir::new(10);
        let tri = &TRIANGLES[4];
        let (new_pos, new_dir) = local_to_world(pos, dir, RESOLUTION, tri);

        // Should now be just below northern tropic, facing north-west.
        assert_eq!(
            CellPos::default()
            .set_x(2)
            .set_y(RESOLUTION[1] / 2 - 1),
            new_pos
        );
        assert_eq!(Dir::new(8), new_dir);
    }
}
