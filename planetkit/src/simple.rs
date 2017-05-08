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

/// `World`-global resource for finding the current entity being controlled
/// by the player.
//
// TODO: find somewhere proper for this. Perhaps a "ActiveCellDweller"
// 1-tuple/newtype.
pub struct ControlledEntity {
    pub entity: specs::Entity,
}

/// Create a new simple PlanetKit app and window.
///
/// Uses all default settings, and logs to standard output.
pub fn new() -> (app::App, PistonWindow) {
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

    // TODO: find a home for this
    let axes_mesh_handle = app.new_axes_mesh();

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

        let movement_sys = cell_dweller::MovementSystem::new(
            movement_input_receiver,
            &log,
        );
        planner.add_system(movement_sys, "cd_movement", prio::CD_MOVEMENT);

        let mining_sys = cell_dweller::MiningSystem::new(
            mining_input_receiver,
            &log,
        );
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

        // Populate the world.
        let world = planner.mut_world();
        let globe_entity = create_simple_globe_now(world);
        let player_character_entity = create_simple_player_character_now(world, globe_entity, axes_mesh_handle);
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

pub fn create_simple_player_character_now(world: &mut specs::World, globe_entity: specs::Entity, axes_mesh_handle: render::MeshHandle) -> specs::Entity {
    use rand::{ XorShiftRng, SeedableRng };
    use specs::Gate;

    // Find globe surface and put player character on it.
    use globe::Dir;
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
    let mut cell_dweller_visual = render::Visual::new_empty();
    cell_dweller_visual.set_mesh_handle(axes_mesh_handle);
    let player_character_entity = world.create_now()
        .with(cell_dweller::CellDweller::new(
            player_character_pos,
            Dir::default(),
            globe_spec,
            Some(globe_entity),
        ))
        .with(cell_dweller_visual)
        // The CellDweller's transformation will be set based
        // on its coordinates in cell space.
        .with(::Spatial::new(globe_entity, Iso3::identity()))
        .build();
    // TODO: make something else register this always, as an Option,
    // so you can just assume it's there and update it.
    world.add_resource(::simple::ControlledEntity {
        entity: player_character_entity,
    });
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
