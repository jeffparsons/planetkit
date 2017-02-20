use std::sync::mpsc;

use slog;
use globe;
use specs;
use cell_dweller;

// TODO: make a proper test harness using `App`, `piston::window::NoWindow`,
// and some custom systems to drive the tests.

#[test]
fn random_walk() {
    // Log to nowhere.
    let drain = slog::Discard;
    let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));

    // Create Specs `World`.
    let mut world = specs::World::new();

    // Register all component types.
    world.register::<::cell_dweller::CellDweller>();
    world.register::<::Spatial>();
    world.register::<::globe::Globe>();

    // Create systems.
    use ::system_priority as prio;

    let chunk_sys = globe::ChunkSystem::new(
        &root_log,
    );

    let (movement_input_sender, movement_input_receiver) = mpsc::channel();
    let movement_sys = cell_dweller::MovementSystem::new(
        movement_input_receiver,
        &root_log,
    );

    // This isn't actually used yet.
    let physics_sys = cell_dweller::PhysicsSystem::new(
        &root_log,
        0.1, // Seconds between falls
    );

    // Hand the world off to a Specs `Planner`.
    let mut planner = specs::Planner::new(world, 2);
    planner.add_system(chunk_sys, "chunk", prio::CHUNK);
    planner.add_system(movement_sys, "cd_movement", prio::CD_MOVEMENT);
    planner.add_system(physics_sys, "cd_physics", prio::CD_PHYSICS);

    // Make a flat globe to prevent the CellDweller from ever getting stuck.
    // TODO: actually do this.
    // REVISIT: make the heigh vary by 1, to test gravity / climbing.
    let globe = globe::Globe::new_example(&root_log);
    // First add the globe to the world so we can get a handle on its entity.
    let globe_spec = globe.spec();
    let globe_entity = planner.mut_world().create_now()
        .with(globe)
        .build();

    // Step the world once before adding our character;
    // otherwise there won't be any chunks so we won't know where to put him!
    planner.dispatch(0.02);
    planner.wait();

    // Find globe surface and put player character on it.
    use globe::{ CellPos, Dir };
    use globe::chunk::Material;
    let mut guy_pos = CellPos::default();
    guy_pos = {
        let globes = planner
            .mut_world()
            .read::<globe::Globe>();
        let globe = globes
            .get(globe_entity)
            .expect("Uh oh, where did our Globe go?");
        globe.find_lowest_cell_containing(guy_pos, Material::Air)
            .expect("Uh oh, there's something wrong with our globe.")
    };
    planner.mut_world().create_now()
        .with(cell_dweller::CellDweller::new(
            guy_pos,
            Dir::default(),
            globe_spec,
            Some(globe_entity),
        ))
        .with(::Spatial::root())
        .build();

    // Start our CellDweller moving forward indefinitely.
    use cell_dweller::MovementEvent;
    movement_input_sender.send(MovementEvent::StepForward(true)).unwrap();

    use rand;
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for _ in 0..10000 {
        // Maybe turn left or right.
        let f: f32 = rng.gen();
        if f < 0.02 {
            // Turn left.
            movement_input_sender.send(MovementEvent::TurnLeft(true)).unwrap();
            movement_input_sender.send(MovementEvent::TurnRight(false)).unwrap();
        } else if f < 0.01 {
            // Turn right.
            movement_input_sender.send(MovementEvent::TurnLeft(false)).unwrap();
            movement_input_sender.send(MovementEvent::TurnRight(true)).unwrap();
        } else {
            // Walk straight.
            movement_input_sender.send(MovementEvent::TurnLeft(false)).unwrap();
            movement_input_sender.send(MovementEvent::TurnRight(false)).unwrap();
        }

        planner.dispatch(0.02);
    }
}
