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
    world: specs::World,
    dispatcher: specs::Dispatcher<'static, 'static>,
    guy_entities: Vec<specs::Entity>,
}

impl Walker {
    pub fn new(walker_count: u16) -> Walker {
        // Log to nowhere.
        let drain = slog::Discard;
        let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));

        // Create Specs `World`.
        let mut world = specs::World::new();

        // Register all component types.
        world.register::<::cell_dweller::CellDweller>();
        world.register::<::Spatial>();
        world.register::<::globe::Globe>();
        world.register::<::net::NetMarker>();

        // Ensure some resources we use are present; we're going to mess
        // around with it before it is automatically ensured through
        // normal System runs.
        //
        // TODO: We should actually just call setup on our dispatcher
        // before we poke things in; that'd be less fragile.
        // Do this across the board wherever you're doing `world.setup`!
        world.setup::<specs::Read<TimeDeltaResource>>();
        world.setup::<specs::Read<cell_dweller::ActiveCellDweller>>();
        world.setup::<specs::Read<cell_dweller::SendMessageQueue>>();

        // Create systems.
        let chunk_sys = globe::ChunkSystem::new(&root_log);

        let (movement_input_sender, movement_input_receiver) = mpsc::channel();
        let mut movement_sys =
            cell_dweller::MovementSystem::new(movement_input_receiver, &root_log);
        // Stop the player from getting stuck on cliffs; we want to test what
        // happens when they walk really aggressively all around the world, not what
        // happens when they fall into a hole and don't move anywhere.
        movement_sys.set_step_height(100);

        let physics_sys = cell_dweller::PhysicsSystem::new(
            &root_log,
            0.1, // Seconds between falls
        );

        // Make a dispatcher and add all our systems.
        let dispatcher = specs::DispatcherBuilder::new()
            .with(movement_sys, "cd_movement", &[])
            .with(physics_sys, "cd_physics", &[])
            .with(chunk_sys, "chunk", &[])
            .build();

        // Use an Earth-scale globe to make it likely we're constantly
        // visiting new chunks.
        let globe = globe::Globe::new_earth_scale_example();
        // First add the globe to the world so we can get a handle on its entity.
        let globe_spec = globe.spec();
        let globe_entity = world.create_entity().with(globe).build();

        // Find globe surface and put player character on it.
        use grid::{GridPoint3, Dir};
        use globe::chunk::Material;
        let mut guy_pos = GridPoint3::default();
        guy_pos = {
            let mut globes = world.write_storage::<globe::Globe>();
            let globe = globes.get_mut(globe_entity).expect(
                "Uh oh, where did our Globe go?",
            );
            globe.find_lowest_cell_containing(guy_pos, Material::Air)
        };

        let guy_entities: Vec<_> = (0..walker_count).map(|_| {
            world
                .create_entity()
                .with(cell_dweller::CellDweller::new(
                    guy_pos,
                    Dir::default(),
                    globe_spec,
                    Some(globe_entity),
                ))
                .with(::Spatial::new_root())
                .build()
        }).collect();

        Walker {
            movement_input_sender: movement_input_sender,
            world: world,
            dispatcher: dispatcher,
            guy_entities: guy_entities,
        }
    }

    // TODO: track how many steps have actually been taken somehow?
    pub fn tick_lots(&mut self, ticks: usize) {
        // Start our CellDweller moving forward indefinitely.
        use cell_dweller::MovementEvent;
        self.movement_input_sender
            .send(MovementEvent::StepForward(true))
            .unwrap();

        use rand;
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..ticks {
            for guy_entity in &self.guy_entities {
                // Set our new character as the currently controlled cell dweller.
                self.world
                    .write_resource::<cell_dweller::ActiveCellDweller>()
                    .maybe_entity = Some(guy_entity.clone());

                // Maybe turn left or right.
                let f: f32 = rng.gen();
                if f < 0.02 {
                    // Turn left.
                    self.movement_input_sender
                        .send(MovementEvent::TurnLeft(true))
                        .unwrap();
                    self.movement_input_sender
                        .send(MovementEvent::TurnRight(false))
                        .unwrap();
                } else if f < 0.01 {
                    // Turn right.
                    self.movement_input_sender
                        .send(MovementEvent::TurnLeft(false))
                        .unwrap();
                    self.movement_input_sender
                        .send(MovementEvent::TurnRight(true))
                        .unwrap();
                } else {
                    // Walk straight.
                    self.movement_input_sender
                        .send(MovementEvent::TurnLeft(false))
                        .unwrap();
                    self.movement_input_sender
                        .send(MovementEvent::TurnRight(false))
                        .unwrap();
                }

                self.world.write_resource::<TimeDeltaResource>().0 = 0.1;
                self.dispatcher.dispatch(&mut self.world.res);
                self.world.maintain();
            }
        }
    }
}

#[test]
fn random_walk_one_walker() {
    use grid::GridPoint3;

    let mut walker = Walker::new(1);
    walker.tick_lots(1000);

    // Walking should have taken us away from the origin.
    assert_eq!(walker.guy_entities.len(), 1);
    let guy_entity = walker.guy_entities.first().unwrap();
    let cd_storage = walker.world.read_storage::<::cell_dweller::CellDweller>();
    let cd = cd_storage.get(guy_entity.clone()).unwrap();
    assert_ne!(cd.pos, GridPoint3::default());
}

#[test]
fn random_walk_three_walkers() {
    use grid::GridPoint3;

    let mut walker = Walker::new(3);
    walker.tick_lots(1000);

    // Walking should have taken us away from the origin.
    assert_eq!(walker.guy_entities.len(), 3);
    for guy_entity in &walker.guy_entities {
        let cd_storage = walker.world.read_storage::<::cell_dweller::CellDweller>();
        let cd = cd_storage.get(guy_entity.clone()).unwrap();
        assert_ne!(cd.pos, GridPoint3::default());
    }
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
    fn bench_random_walk_one_walker(b: &mut Bencher) {
        let mut walker = Walker::new(1);
        // Start by moving everyone away from the origin.
        walker.tick_lots(1000);
        b.iter(|| { walker.tick_lots(10); });
    }

    // These multi-walker tests are to make sure we don't get pathological performance
    // from having multiple "points of interest" on the globe. (Thrashing loading and unloading
    // chunks as each walker moves.)
    //
    // No prizes for guessing how this test was born... :)

    #[bench]
    fn bench_random_walk_two_walkers(b: &mut Bencher) {
        let mut walker = Walker::new(2);
        // Start by moving everyone away from the origin.
        walker.tick_lots(1000);
        b.iter(|| { walker.tick_lots(10); });
    }

    #[bench]
    fn bench_random_walk_three_walkers(b: &mut Bencher) {
        let mut walker = Walker::new(3);
        // Start by moving everyone away from the origin.
        walker.tick_lots(1000);
        b.iter(|| { walker.tick_lots(10); });
    }
}
