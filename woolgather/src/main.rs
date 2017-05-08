extern crate planetkit as pk;
extern crate specs;

mod shepherd;

fn main() {
    let (mut app, mut window) = pk::simple::new();
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
    let globe = pk::globe::Globe::new_earth_scale_example();
    let globe_spec = globe.spec();
    let globe_entity = world.create_now()
        .with(globe)
        .with(pk::Spatial::new_root())
        .build();

    // Create the shepherd.
    // TODO: Use `create_later_build`, now that it exists?
    let shepherd_entity = shepherd::create_now(world, globe_entity, globe_spec);
    // Set our new shepherd player character as the currently controlled cell dweller.
    world.write_resource::<ActiveCellDweller>().pass().maybe_entity =
        Some(shepherd_entity);
}
