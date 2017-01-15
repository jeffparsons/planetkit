use super::*;
use ::globe::{ CellPos, Dir };
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
fn move_forward_into_northern_tropic_pentagon() {
    // Start facing east, just west of a northern tropic pentagon.
    let mut pos = CellPos::default()
        .set_x(1)
        .set_y(RESOLUTION[0] - 1);
    let mut dir = Dir::new(4);

    move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

    // We should now be sitting on the northern tropic pentagon,
    // facing south-east in root 1.
    //
    // Note that it wouldn't be legal to step in this direction.
    assert_eq!(
        CellPos::default()
            .set_root(1)
            .set_x(RESOLUTION[0]),
        pos
    );
    assert_eq!(Dir::new(3), dir);

    // Turn around, and walk back! Note some hacks to get back to a
    // legal movement direction for now... and then using the smart
    // turning functions to handle rebasing.
    dir = Dir::new(dir.index + 1);
    turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();
    turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();

    move_forward(&mut pos, &mut dir, RESOLUTION).unwrap();

    // We should now be back where we started, but facing west.
    assert_eq!(
        CellPos::default()
            .set_x(1)
            .set_y(RESOLUTION[0] - 1),
        pos
    );
    assert_eq!(Dir::new(10), dir);

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

#[test]
fn random_walks() {
    use rand;
    use rand::Rng;

    // Try to simulate all the kinds of stepping and turning that a
    // CellDweller might actually be able to do in the real world.
    //
    // Use a small resolution; we want to very quickly stumble
    // upon pathological cases.
    const RESOLUTION: [i64; 2] = [4, 8];
    // Multiple walks will reveal problems that one long walk won't;
    // e.g. forgetting to switch the turn bias if on a pentagon when turning
    // around at the end of the walk. (Yes, I forgot this, and it made this
    // test intermittently fail. Hodor.)
    const WALKS: usize = 100;
    const STEPS: usize = 100;

    // Keep a list of the opposite steps we'll need to take to get
    // back home.
    enum Action {
        StepForward,
        TurnLeft,
        TurnRight,
    }

    for _ in 0..WALKS {
        let mut rng = rand::thread_rng();

        // Start at (0, 0). This is not a very interesting place to start, but we'll
        // be randomly walking all over the place, so there shouldn't be any need for
        // the starting point to be interesting.
        let mut pos = CellPos::default();
        let mut dir = Dir::default();
        let mut last_turn_bias = TurnDir::Left;

        let mut crumbs: Vec<Action> = Vec::new();
        for _ in 0..STEPS {
            // Consider turning several times before stepping,
            // with low probability on each.
            for _ in 0..10 {
                let f: f32 = rng.gen();
                if f < 0.02 {
                    turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();
                    crumbs.push(Action::TurnLeft);
                    println!("Turned left.");

                } else if f < 0.01 {
                    turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();
                    crumbs.push(Action::TurnRight);
                    println!("Turned right.");
                }
            }

            step_forward_and_face_neighbor(
                &mut pos,
                &mut dir,
                RESOLUTION,
                &mut last_turn_bias
            ).unwrap();
            crumbs.push(Action::StepForward);
            println!("Stepped forward: {:?}", pos);
        }

        // Aaaand turn around and walk back home!
        turn_around_and_face_neighbor(&mut pos, &mut dir, RESOLUTION, last_turn_bias);
        if is_pentagon(&pos, RESOLUTION) {
            // Update turn bias; if we walk forward again, we want a _repeat_
            // of the movement we just un-did.
            last_turn_bias = last_turn_bias.opposite();
        }

        // Retrace steps.
        crumbs.reverse();
        for crumb in crumbs {
            match crumb {
                Action::StepForward => step_forward_and_face_neighbor(&mut pos, &mut dir, RESOLUTION, &mut last_turn_bias).unwrap(),
                Action::TurnLeft => turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap(),
                Action::TurnRight => turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap(),
            }
            println!("Retracing crumbs walking forward; now at: {:?}, {:?}", pos, dir);
        }

        // We should now be back at the start, but re-based into another root,
        // facing down one of its axes.
        //
        // Note that depending on the number of STEPS, whether we turned one
        // or more times at the first step, and therefore via which root we
        // arrive home, both the root and the direction will vary.
        assert_eq!(0, pos.x);
        assert_eq!(0, pos.y);
    }
}

#[test]
fn random_walks_retraced_by_stepping_backwards() {
    use rand;
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Try to simulate all the kinds of stepping and turning that a
    // CellDweller might actually be able to do in the real world.
    //
    // Use a small resolution; we want to very quickly stumble
    // upon pathological cases.
    const RESOLUTION: [i64; 2] = [4, 8];
    // Multiple walks will reveal problems that one long walk won't;
    // e.g. forgetting to switch the turn bias if on a pentagon when turning
    // around at the end of the walk. (Yes, I forgot this, and it made this
    // test intermittently fail. Hodor.)
    const WALKS: usize = 100;
    const STEPS: usize = 100;

    // Keep a list of the opposite steps we'll need to take to get
    // back home.
    enum Action {
        StepBackward,
        TurnLeft,
        TurnRight,
    }

    for _ in 0..WALKS {
        // Start at (0, 0). This is not a very interesting place to start, but we'll
        // be randomly walking all over the place, so there shouldn't be any need for
        // the starting point to be interesting.
        let mut pos = CellPos::default();
        let mut dir = Dir::default();
        let mut last_turn_bias = TurnDir::Left;

        let mut crumbs: Vec<Action> = Vec::new();

        for _ in 0..STEPS {
            // Consider turning several times before stepping,
            // with low probability on each.
            for _ in 0..10 {
                let f: f32 = rng.gen();
                if f < 0.02 {
                    turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();
                    crumbs.push(Action::TurnLeft);
                    println!("Turned left.");
                } else if f < 0.01 {
                    turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap();
                    crumbs.push(Action::TurnRight);
                    println!("Turned right.");
                }
            }

            step_forward_and_face_neighbor(
                &mut pos,
                &mut dir,
                RESOLUTION,
                &mut last_turn_bias
            ).unwrap();
            crumbs.push(Action::StepBackward);
            println!("Stepped forward: {:?}", pos);
        }

        // Aaaand start walking backwards to find home!
        crumbs.reverse();
        for crumb in crumbs {
            match crumb {
                Action::StepBackward => step_backward_and_face_neighbor(&mut pos, &mut dir, RESOLUTION, &mut last_turn_bias).unwrap(),
                Action::TurnLeft => turn_left_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap(),
                Action::TurnRight => turn_right_by_one_hex_edge(&mut pos, &mut dir, RESOLUTION).unwrap(),
            }
            println!("Retracing crumbs walking backward; now at: {:?}, {:?}", pos, dir);
        }

        // We should now be back at the start, but re-based into another root,
        // facing down one of its axes.
        //
        // Note that depending on the number of STEPS, whether we turned one
        // or more times at the first step, and therefore via which root we
        // arrive home, both the root and the direction will vary.
        assert_eq!(0, pos.x);
        assert_eq!(0, pos.y);
    }
}
