use std::sync::mpsc;

use slog;
use globe;
use specs;
use specs::Gate;
use cell_dweller;
use types::*;

// TODO: make a proper test harness using `App`, `piston::window::NoWindow`,
// and some custom systems to drive the tests.
struct Walker {
    movement_input_sender: mpsc::Sender<cell_dweller::MovementEvent>,
    planner: specs::Planner<TimeDelta>,
    guy_entity: specs::Entity,
}

impl Walker {
    pub fn new() -> Walker {
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
        let mut movement_sys = cell_dweller::MovementSystem::new(
            movement_input_receiver,
            &root_log,
        );
        movement_sys.init(&mut world);
        // Stop the player from getting stuck on cliffs; we want to test what
        // happens when they walk really aggressively all around the world, not what
        // happens when they fall into a hole and don't move anywhere.
        movement_sys.set_step_height(100);

        let physics_sys = cell_dweller::PhysicsSystem::new(
            &root_log,
            0.1, // Seconds between falls
        );

        // Hand the world off to a Specs `Planner`.
        let mut planner = specs::Planner::new(world);
        planner.add_system(movement_sys, "cd_movement", prio::CD_MOVEMENT);
        planner.add_system(physics_sys, "cd_physics", prio::CD_PHYSICS);
        planner.add_system(chunk_sys, "chunk", prio::CHUNK);

        // Use an Earth-scale globe to make it likely we're constantly
        // visiting new chunks.
        let globe = globe::Globe::new_earth_scale_example();
        // First add the globe to the world so we can get a handle on its entity.
        let globe_spec = globe.spec();
        let globe_entity = planner.mut_world().create_now()
            .with(globe)
            .build();

        // Find globe surface and put player character on it.
        use grid::{ GridPoint3, Dir };
        use globe::chunk::Material;
        let mut guy_pos = GridPoint3::default();
        guy_pos = {
            let mut globes = planner
                .mut_world()
                .write::<globe::Globe>()
                .pass();
            let mut globe = globes
                .get_mut(globe_entity)
                .expect("Uh oh, where did our Globe go?");
            globe.find_lowest_cell_containing(guy_pos, Material::Air)
        };
        let guy_entity = planner.mut_world().create_now()
            .with(cell_dweller::CellDweller::new(
                guy_pos,
                Dir::default(),
                globe_spec,
                Some(globe_entity),
            ))
            .with(::Spatial::new_root())
            .build();
        // Set our new character as the currently controlled cell dweller.
        planner.mut_world().write_resource::<cell_dweller::ActiveCellDweller>().pass().maybe_entity =
            Some(guy_entity);

        Walker {
            movement_input_sender: movement_input_sender,
            planner: planner,
            guy_entity: guy_entity,
        }
    }

    // TODO: track how many steps have actually been taken somehow?
    pub fn tick_lots(&mut self, ticks: usize) {
        // Start our CellDweller moving forward indefinitely.
        use cell_dweller::MovementEvent;
        self.movement_input_sender.send(MovementEvent::StepForward(true)).unwrap();

        use rand;
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..ticks {
            // Maybe turn left or right.
            let f: f32 = rng.gen();
            if f < 0.02 {
                // Turn left.
                self.movement_input_sender.send(MovementEvent::TurnLeft(true)).unwrap();
                self.movement_input_sender.send(MovementEvent::TurnRight(false)).unwrap();
            } else if f < 0.01 {
                // Turn right.
                self.movement_input_sender.send(MovementEvent::TurnLeft(false)).unwrap();
                self.movement_input_sender.send(MovementEvent::TurnRight(true)).unwrap();
            } else {
                // Walk straight.
                self.movement_input_sender.send(MovementEvent::TurnLeft(false)).unwrap();
                self.movement_input_sender.send(MovementEvent::TurnRight(false)).unwrap();
            }

            self.planner.dispatch(0.1);
        }
    }
}

#[test]
fn random_walk() {
    use grid::GridPoint3;

    let mut walker = Walker::new();
    walker.tick_lots(10000);

    // Walking should have taken us away from the origin.
    let guy_entity = walker.guy_entity;
    let world = walker.planner.mut_world();
    let cd_storage = world.read::<::cell_dweller::CellDweller>().pass();
    let cd = cd_storage.get(guy_entity).unwrap();
    assert_ne!(cd.pos, GridPoint3::default());
}

#[cfg(feature = "nightly")]
pub mod benches {
    use test::Bencher;

    use super::*;

    #[bench]
    // # History for random walks:
    //
    // - Earth-scale globe, with initial implementation of unloading most
    //   distant chunks when you have too many loaded.
    //     - 185,350,057 ns/iter (+/- 128,603,998)
    //
    // NOTE: avoid premature fiddly optimisation through clever caching, or anything that makes
    // the design harder to work with; rather go for the optimisations that push all this
    // forward in general and make interfaces _more elegant_.
    fn bench_random_walk(b: &mut Bencher) {
        let mut walker = Walker::new();
        b.iter(|| {
            walker.tick_lots(100);
        });
    }
}
