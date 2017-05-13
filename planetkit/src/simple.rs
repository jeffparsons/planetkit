use std::sync::mpsc;

use piston_window::PistonWindow;

use slog;
use slog_term;
use slog_async;
use specs;

use types::*;
use window;
use app;
use globe;
use cell_dweller;
use render;

/// Create a new simple PlanetKit app and window.
///
/// Uses all default settings, logs to standard output, and registers most
/// of the systems you're likely to want to use.
pub fn new_empty() -> (app::App, PistonWindow) {
    use slog::Drain;
    use super::system_priority as prio;

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));
    let log = root_log;

    let mut window = window::make_window(&log);
    let mut app = app::App::new(&log, &mut window);

    // Set up input adapters.
    use cell_dweller;
    let (movement_input_sender, movement_input_receiver) = mpsc::channel();
    let movement_input_adapter = cell_dweller::MovementInputAdapter::new(movement_input_sender);
    app.add_input_adapter(Box::new(movement_input_adapter));

    let (mining_input_sender, mining_input_receiver) = mpsc::channel();
    let mining_input_adapter = cell_dweller::MiningInputAdapter::new(mining_input_sender);
    app.add_input_adapter(Box::new(mining_input_adapter));

    {
        let planner = app.planner();

        {
            // Register all component types.
            let world = planner.mut_world();
            world.register::<::cell_dweller::CellDweller>();
            world.register::<::render::Visual>();
            world.register::<::Spatial>();
            world.register::<::globe::Globe>();
            world.register::<::globe::ChunkView>();
        }

        // Initialize all systems.
        // TODO: split out system initialization into helper functions.

        let mut movement_sys = cell_dweller::MovementSystem::new(
            movement_input_receiver,
            &log,
        );
        movement_sys.init(planner.mut_world());
        planner.add_system(movement_sys, "cd_movement", prio::CD_MOVEMENT);

        let mut mining_sys = cell_dweller::MiningSystem::new(
            mining_input_receiver,
            &log,
        );
        mining_sys.init(planner.mut_world());
        planner.add_system(mining_sys, "cd_mining", prio::CD_MINING);

        let physics_sys = cell_dweller::PhysicsSystem::new(
            &log,
            0.1, // Seconds between falls
        );
        planner.add_system(physics_sys, "cd_physics", prio::CD_PHYSICS);

        use globe;
        let chunk_sys = globe::ChunkSystem::new(
            &log,
        );
        planner.add_system(chunk_sys, "chunk", prio::CHUNK);

        let chunk_view_sys = globe::ChunkViewSystem::new(
            &log,
            0.05, // Seconds between geometry creation
        );
        planner.add_system(chunk_view_sys, "chunk_view", prio::CHUNK_VIEW);
    }

    (app, window)
}

/// Create a new simple PlanetKit app and window with some example entities.
///
/// Creates a world using `new_empty` then populates it with some entities.
/// Hack first, ask questions later.
pub fn new_populated() -> (app::App, PistonWindow) {
    let (mut app, window) = new_empty();
    // Populate the world.
    {
        let world = app.planner().mut_world();
        let globe_entity = create_simple_globe_now(world);
        let player_character_entity = create_simple_player_character_now(world, globe_entity);
        create_simple_chase_camera_now(world, player_character_entity);
    }
    (app, window)
}

pub fn create_simple_globe_now(world: &mut specs::World) -> specs::Entity {
    let globe = globe::Globe::new_earth_scale_example();
    world.create_now()
        .with(globe)
        .with(::Spatial::new_root())
        .build()
}

pub fn create_simple_player_character_now(world: &mut specs::World, globe_entity: specs::Entity) -> specs::Entity {
    use rand::{ XorShiftRng, SeedableRng };
    use specs::Gate;

    // Find a suitable spawn point for the player character at the globe surface.
    use grid::Dir;
    let (globe_spec, player_character_pos) = {
        let mut globe_storage = world.write::<globe::Globe>().pass();
        let globe = globe_storage.get_mut(globe_entity)
            .expect("Uh oh, it looks like our Globe went missing.");
        let globe_spec = globe.spec();
        // Seed spawn point RNG with world seed.
        let seed = globe_spec.seed;
        let mut rng = XorShiftRng::from_seed([seed, seed, seed, seed]);
        let player_character_pos = globe.air_above_random_surface_dry_land(
            &mut rng,
            2, // Min air cells above
            5, // Max distance from starting point
            5, // Max attempts
        ).expect("Oh noes, we took too many attempts to find a decent spawn point!");
        (globe_spec, player_character_pos)
    };

    // Make visual appearance of player character.
    // For now this is just an axes mesh.
    let mut player_character_visual = render::Visual::new_empty();
    player_character_visual.proto_mesh = Some(render::make_axes_mesh());

    let player_character_entity = world.create_now()
        .with(cell_dweller::CellDweller::new(
            player_character_pos,
            Dir::default(),
            globe_spec,
            Some(globe_entity),
        ))
        .with(player_character_visual)
        // The CellDweller's transformation will be set based
        // on its coordinates in cell space.
        .with(::Spatial::new(globe_entity, Iso3::identity()))
        .build();
    // Set our new character as the currently controlled cell dweller.
    world.write_resource::<cell_dweller::ActiveCellDweller>().pass().maybe_entity =
        Some(player_character_entity);
    player_character_entity
}

pub fn create_simple_chase_camera_now(world: &mut specs::World, player_character_entity: specs::Entity) -> specs::Entity {
    // Create a camera sitting a little bit behind the cell dweller.
    let eye = Pt3::new(0.0, 4.0, -6.0);
    let target = Pt3::origin();
    let camera_transform = Iso3::new_observer_frame(&eye, &target, &Vec3::z());
    let camera_entity = world.create_now()
        .with(::Spatial::new(player_character_entity, camera_transform))
        .build();
    use ::camera::DefaultCamera;
    // TODO: gah, where does this belong?
    world.add_resource(DefaultCamera {
        camera_entity: camera_entity,
    });
    camera_entity
}
