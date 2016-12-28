use super::*;
use super::super::{ CellPos, Dir };
use super::triangles::TRIANGLES;

const RESOLUTION: [i64; 2] = [32, 64];

#[test]
fn move_forward_in_positive_x_direction() {
    let mut pos = CellPos::default();
    let mut dir = Dir::default();
    move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();
    assert_eq!(CellPos::default().set_x(1), pos);
    assert_eq!(Dir::default(), dir);
}

#[test]
fn turn_left_at_northern_tropic() {
    let triangle = &TRIANGLES[2];
    // Start at triangle apex.
    // Both parts of the apex are expressed in terms of x-dimension.
    let apex = triangle.apex * RESOLUTION[0];
    let mut pos = CellPos::default()
        .set_root(0)
        .set_x(apex.x)
        .set_y(apex.y);
    let mut dir = Dir::new(triangle.x_dir);

    // Should be facing north in root 0.
    assert_eq!(0, pos.root.index);
    assert_eq!(Dir::new(8), dir);

    turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing west in root 0.
    assert_eq!(0, pos.root.index);
    assert_eq!(Dir::new(10), dir);

    turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing south-west in root 0.
    assert_eq!(0, pos.root.index);
    assert_eq!(Dir::new(0), dir);

    turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing south-east in root 0.
    assert_eq!(0, pos.root.index);
    assert_eq!(Dir::new(2), dir);

    turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing east in root 1.
    assert_eq!(1, pos.root.index);
    assert_eq!(Dir::new(4), dir);

    turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing north in root 1.
    // Note that this represents where we started, but we should be
    // stable in the same root we just came from instead of unnecessarily
    // rebasing on the neighbour.
    assert_eq!(1, pos.root.index);
    assert_eq!(Dir::new(6), dir);

    turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing west in root 0.
    // This is a repeat of the first turn we made, but now we're
    // coming in from a neighbouring root rather than starting in root 0.
    assert_eq!(0, pos.root.index);
    assert_eq!(Dir::new(10), dir);
}

#[test]
fn turn_right_at_northern_tropic() {
    let triangle = &TRIANGLES[2];
    // Start at triangle apex.
    // Both parts of the apex are expressed in terms of x-dimension.
    let apex = triangle.apex * RESOLUTION[0];
    let mut pos = CellPos::default()
        .set_root(0)
        .set_x(apex.x)
        .set_y(apex.y);
    let mut dir = Dir::new(triangle.x_dir);

    // Should be facing north in root 0.
    assert_eq!(0, pos.root.index);
    assert_eq!(Dir::new(8), dir);

    turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing east in root 1.
    assert_eq!(1, pos.root.index);
    assert_eq!(Dir::new(4), dir);

    turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing south-east in root 1.
    assert_eq!(1, pos.root.index);
    assert_eq!(Dir::new(2), dir);

    turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing south-west in root 0.
    assert_eq!(0, pos.root.index);
    assert_eq!(Dir::new(0), dir);

    turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing west in root 0.
    assert_eq!(0, pos.root.index);
    assert_eq!(Dir::new(10), dir);

    turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    // Should be facing north in root 0.
    // Note that this represents where we started, but unlike `turn_left_at_northern_tropic`,
    // this will be the exact same position as we started in -- not a different representation.
    // This is because the other test has a way of representing the starting angle from a
    // different quad, because it approaches it from a different root that shares that angle.
    assert_eq!(0, pos.root.index);
    assert_eq!(Dir::new(8), dir);
}

#[test]
fn move_east_under_north_pole() {
    // Start just south of the north pole in root 4,
    // facing north-east.
    let mut pos = CellPos::default()
        .set_root(4)
        .set_x(1)
        .set_y(1);
    let mut dir = Dir::new(6);
    move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

    // We should now be on the edge of root 4 and 0,
    // facing east into root 0.
    assert_eq!(CellPos::default().set_x(1), pos);
    assert_eq!(Dir::new(4), dir);

    move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

    // We should now be on the edge of root 0 and 1,
    // facing south-east into root 1.
    assert_eq!(CellPos::default().set_root(1).set_x(1), pos);
    assert_eq!(Dir::new(2), dir);

    move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

    // We should now be just south of the north pole in root 1.
    assert_eq!(CellPos::default().set_root(1).set_x(1).set_y(1), pos);
    assert_eq!(Dir::new(2), dir);
}

#[test]
fn move_west_under_north_pole() {
    // Start just south of the north pole in root 1,
    // facing north-west.
    let mut pos = CellPos::default()
        .set_root(1)
        .set_x(1)
        .set_y(1);
    let mut dir = Dir::new(8);
    move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

    // We should now be on the edge of root 1 and 0,
    // facing west into root 0.
    assert_eq!(CellPos::default().set_y(1), pos);
    assert_eq!(Dir::new(10), dir);

    move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

    // We should now be on the edge of root 0 and 4,
    // facing south-west into root 1.
    assert_eq!(CellPos::default().set_root(4).set_y(1), pos);
    assert_eq!(Dir::new(0), dir);

    move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

    // We should now be just south of the north pole in root 4.
    assert_eq!(CellPos::default().set_root(4).set_x(1).set_y(1), pos);
    assert_eq!(Dir::new(0), dir);
}

#[test]
fn walk_anticlockwise_around_all_pentagons() {
    // For each triangle in each root, start at its apex, take one step
    // out along its x-axis, and then walk around in a circle just beyond
    // the pentagon until we're back at the first hexagon we visited.
    let triangle_indexes: Vec<usize> = (0..12).collect();
    for root_index in 0..5 {
        for triangle_index in triangle_indexes.iter() {
            println!("Starting in root {} at apex of triangle {}.", root_index, triangle_index);
            let triangle = &TRIANGLES[*triangle_index];

            // Start at triangle apex.
            // Both parts of the apex are expressed in terms of x-dimension.
            let apex = triangle.apex * RESOLUTION[0];
            let mut pos = CellPos::default()
                .set_root(root_index)
                .set_x(apex.x)
                .set_y(apex.y);
            let mut dir = Dir::new(triangle.x_dir);

            // Take one step out along the x-axis and then face towards
            // the next hexagon moving in an anticlockwise circle around
            // the starting pentagon.
            move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();
            turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();
            turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

            // Remember where we're supposed to end up.
            let final_pos = pos.clone();
            let final_dir = dir.clone();

            for _ in 0..5 {
                // Step forward. This should land us in the equivalent
                // hexagon in the next root anti-clockwise from here.
                move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

                // Turn left. This will point us back at the next root
                // anti-clockwise from here, leaving us ready to step again.
                turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();
            }

            // We should now be back at the first hexagon we visited.
            assert_eq!(final_pos, pos);
            assert_eq!(final_dir, dir);
        }
    }
}

#[test]
fn walk_clockwise_around_all_pentagons() {
    // For each triangle in each root, start at its apex, take one step
    // out along its x-axis, and then walk around in a circle just beyond
    // the pentagon until we're back at the first hexagon we visited.
    let triangle_indexes: Vec<usize> = (0..12).collect();
    for root_index in 0..5 {
        for triangle_index in triangle_indexes.iter() {
            println!("Starting in root {} at apex of triangle {}.", root_index, triangle_index);
            let triangle = &TRIANGLES[*triangle_index];

            // Start at triangle apex.
            // Both parts of the apex are expressed in terms of x-dimension.
            let apex = triangle.apex * RESOLUTION[0];
            let mut pos = CellPos::default()
                .set_root(root_index)
                .set_x(apex.x)
                .set_y(apex.y);
            let mut dir = Dir::new(triangle.x_dir);

            // Take one step out along the x-axis and then face towards
            // the next hexagon moving in an clockwise circle around
            // the starting pentagon.
            move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();
            turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();
            turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

            // Remember where we're supposed to end up.
            let final_pos = pos.clone();
            let final_dir = dir.clone();

            for _ in 0..5 {
                // Step forward. This should land us in the equivalent
                // hexagon in the next root clockwise from here.
                move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

                // Turn right. This will point us back at the next root
                // clockwise from here, leaving us ready to step again.
                turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();
            }

            // We should now be back at the first hexagon we visited.
            assert_eq!(final_pos, pos);
            assert_eq!(final_dir, dir);
        }
    }
}

// TODO: tests that run over every triangle:
//
// - Walk through pentagon. Turn around. Walk back through.
// - Fuzz tests:
//   - Random walks
//   - Random walks with retracing steps
