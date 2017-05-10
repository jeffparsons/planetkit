extern crate planetkit as pk;
extern crate specs;
extern crate rand;
#[macro_use]
extern crate slog;

mod shepherd;
mod game_state;
mod game_system;

fn main() {
    let (mut app, mut window) = pk::simple::new_empty();
    {
        add_systems(&mut app);
        let world = app.planner().mut_world();
        create_entities(world);
    }
    app.run(&mut window);
}

fn add_systems(app: &mut pk::app::App) {
    app.add_system(|logger| (
        game_system::GameSystem::new(logger),
        "woolgather_game",
        // TODO: figure out how to interleave PlanetKit and application priorities.
        // Constraints would be so much better.
        150,
    ));
}

fn create_entities(world: &mut specs::World) {
    use specs::Gate;
    use pk::cell_dweller::ActiveCellDweller;

    // Create the globe first, because we'll need it to figure out where
    // to place the shepherd (player character).
    let globe_entity = pk::simple::create_simple_globe_now(world);

    // Create the shepherd.
    let shepherd_entity = shepherd::create_now(world, globe_entity);
    // Set our new shepherd player character as the currently controlled cell dweller.
    world.write_resource::<ActiveCellDweller>().pass().maybe_entity =
        Some(shepherd_entity);

    // Create basic third-person following camera.
    pk::simple::create_simple_chase_camera_now(world, shepherd_entity);
}
