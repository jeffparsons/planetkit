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
    // Register shepherd as currently controlled cell dweller.
    // Overwrite it, because App sets it. TODO: just add it after you gut App?
    // Or have it always present, and as an Option<_>?
    *world.write_resource::<pk::simple::ControlledEntity>().pass() =
        pk::simple::ControlledEntity {
            entity: shepherd_entity,
        };
}
