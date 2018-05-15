extern crate planetkit as pk;
extern crate shred;
extern crate specs;
extern crate rand;
#[macro_use]
extern crate slog;

mod shepherd;
mod game_state;
mod game_system;

fn main() {
    let mut app = pk::AppBuilder::new()
        .with_common_systems()
        .with_systems(add_systems)
        .build_gui();
    create_entities(app.world_mut());
    app.run();
}

fn add_systems(
    logger: &slog::Logger,
    world: &mut specs::World,
    dispatcher_builder: specs::DispatcherBuilder<'static, 'static>,
) -> specs::DispatcherBuilder<'static, 'static> {
    use game_state::GameState;
    GameState::ensure_registered(world);

    let game_system = game_system::GameSystem::new(logger);
    dispatcher_builder.with(game_system, "woolgather_game", &[])
}

fn create_entities(world: &mut specs::World) {
    use pk::cell_dweller::ActiveCellDweller;

    // TODO: this should all actually be done by a game system,
    // rather than in the app builder. Because, e.g. if you change levels,
    // it needs to know how to create all this.

    // Create the globe first, because we'll need it to figure out where
    // to place the shepherd (player character).
    let globe_entity = pk::simple::create_simple_globe_now(world);

    // Create the shepherd.
    let shepherd_entity = shepherd::create_now(world, globe_entity);
    // Set our new shepherd player character as the currently controlled cell dweller.
    world.write_resource::<ActiveCellDweller>().maybe_entity = Some(shepherd_entity);

    // Create basic third-person following camera.
    pk::simple::create_simple_chase_camera_now(world, shepherd_entity);
}
