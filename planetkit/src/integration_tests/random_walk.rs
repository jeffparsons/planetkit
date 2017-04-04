use std::sync::mpsc;

use slog;
use globe;
use specs;
use cell_dweller;
use types::*;

// TODO: make a proper test harness using `App`, `piston::window::NoWindow`,
// and some custom systems to drive the tests.
struct Walker {
    movement_input_sender: mpsc::Sender<cell_dweller::MovementEvent>,
    planner: specs::Planner<TimeDelta>,
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
        // Stop the player from getting stuck on cliffs; we want to test what
        // happens when they walk really aggressively all around the world, not what
        // happens when they fall into a hole and don't move anywhere.
        movement_sys.set_step_height(100);

        let physics_sys = cell_dweller::PhysicsSystem::new(
            &root_log,
            0.1, // Seconds between falls
        );

        // Hand the world off to a Specs `Planner`.
        let mut planner = specs::Planner::new(world, 2);
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
        use globe::{ CellPos, Dir };
        use globe::chunk::Material;
        let mut guy_pos = CellPos::default();
        guy_pos = {
            let mut globes = planner
                .mut_world()
                .write::<globe::Globe>();
            let mut globe = globes
                .get_mut(globe_entity)
                .expect("Uh oh, where did our Globe go?");
            globe.find_lowest_cell_containing(guy_pos, Material::Air)
                .expect("Uh oh, there's something wrong with our globe.")
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
        planner.mut_world().add_resource(::simple::ControlledEntity {
            entity: guy_entity,
        });

        Walker {
            movement_input_sender: movement_input_sender,
            planner: planner,
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
    let mut walker = Walker::new();
    walker.tick_lots(10000);
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
