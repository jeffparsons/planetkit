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

    move_forward(&mut pos, &mut dir, resolution).unwrap();

    // We should now be on the edge of root 0 and 1,
    // facing south-east into root 1.
    assert_eq!(CellPos::default().set_root(1).set_x(1), pos);
    assert_eq!(Dir::new(2), dir);

    move_forward(&mut pos, &mut dir, resolution).unwrap();

    // We should now be just south of the north pole in root 1.
    assert_eq!(CellPos::default().set_root(1).set_x(1).set_y(1), pos);
    assert_eq!(Dir::new(2), dir);
}

#[test]
fn move_west_under_north_pole() {
    let resolution: [i64; 2] = [32, 64];

    // Start just south of the north pole in root 1,
    // facing north-west.
    let mut pos = CellPos::default()
        .set_root(1)
        .set_x(1)
        .set_y(1);
    let mut dir = Dir::new(8);
    move_forward(&mut pos, &mut dir, resolution).unwrap();

    // We should now be on the edge of root 1 and 0,
    // facing west into root 0.
    assert_eq!(CellPos::default().set_y(1), pos);
    assert_eq!(Dir::new(10), dir);

    move_forward(&mut pos, &mut dir, resolution).unwrap();

    // We should now be on the edge of root 0 and 4,
    // facing south-west into root 1.
    assert_eq!(CellPos::default().set_root(4).set_y(1), pos);
    assert_eq!(Dir::new(0), dir);

    move_forward(&mut pos, &mut dir, resolution).unwrap();

    // We should now be just south of the north pole in root 4.
    assert_eq!(CellPos::default().set_root(4).set_x(1).set_y(1), pos);
    assert_eq!(Dir::new(0), dir);
}

// TODO: tests that run over every pentagon:
//
// - Step out from pentagon, walk around all neighbor
//   roots in a circle.
// - Walk through pentagon. Turn around. Walk back through.
