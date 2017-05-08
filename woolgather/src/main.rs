extern crate planetkit as pk;
extern crate specs;
extern crate rand;

mod shepherd;

fn main() {
    let (mut app, mut window) = pk::simple::new_empty();
    {
        let mut world = app.planner().mut_world();
        create_entities(&mut world);
    }
    app.run(&mut window);
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
