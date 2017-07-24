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
    let (mut app, mut window) = pk::simple::new_empty(add_systems);
    create_entities(app.world_mut());
    app.run(&mut window);
}

fn add_systems(
    logger: &slog::Logger,
    world: &mut specs::World,
    dispatcher_builder: specs::DispatcherBuilder<'static, 'static>,
) -> specs::DispatcherBuilder<'static, 'static> {
    use game_state::GameState;
    GameState::ensure_registered(world);

    let game_system = game_system::GameSystem::new(logger);
    dispatcher_builder.add(game_system, "woolgather_game", &[])
}

fn create_entities(world: &mut specs::World) {
    use pk::cell_dweller::ActiveCellDweller;

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
