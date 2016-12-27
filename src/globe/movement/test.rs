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

    // TODO: broaden this to the full list of pentagons as you
    // flesh out `triangles.rs`.

    let triangles = [
        &TRIANGLES[0]
    ];

    for root_index in 0..5 {
        for triangle in triangles.iter() {
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
            dir = dir.next_hex_edge_left().next_hex_edge_left();

            // Remember where we're supposed to end up.
            let final_pos = pos.clone();
            let final_dir = dir.clone();

            for _ in 0..5 {
                // Step forward. This should land us in the equivalent
                // hexagon in the next root anti-clockwise from here.
                move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

                // Turn left. This will point us back at the next root
                // anti-clockwise from here, leaving us ready to step again.
                //
                // TODO: don't manipulate this directly; use the root-and-pentagon-aware
                // rotation functions that you haven't written yet!
                dir = dir.next_hex_edge_left();
            }

            // We should now be back at the first hexagon we visited.
            assert_eq!(final_pos, pos);
            assert_eq!(final_dir, dir);
        }
    }
}

// TODO: tests that run over every pentagon:
//
// - Step out from pentagon, walk around all neighbor
//   roots in a circle.
// - Walk through pentagon. Turn around. Walk back through.
