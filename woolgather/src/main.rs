extern crate planetkit as pk;
extern crate specs;

use pk::types::*;

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
    // Create the globe first, because we'll need it to figure out where
    // to place the shepherd (player character).
    use pk::Spatial;
    let globe = pk::globe::Globe::new_earth_scale_example();
    let globe_spec = globe.spec();
    let globe_entity = world.create_now()
        .with(globe)
        .with(pk::Spatial::new_root())
        .build();

    // Create the shepherd.
    // TODO: Needing to have access to the ChunkSystem to place the player character
    // is kind of awkward, but might also be correct. The ChunkSystem might need to load
    // chunks from disk to find an appropriate place, so it does seem like we need a reference
    // to it somehow.
    use pk::globe::{ CellPos, Dir };
    let shepherd_pos = CellPos::default();
    let shepherd_entity = world.create_now()
        .with(pk::cell_dweller::CellDweller::new(
            shepherd_pos,
            Dir::default(),
            globe_spec,
            Some(globe_entity),
        ))
        // The CellDweller's transformation will be set based on its coordinates in cell space.
        .with(Spatial::new(globe_entity, Iso3::identity()))
        .build();
    let shepherd = shepherd::Shepherd::new(shepherd_entity);
    // There is exactly one shepherd in the game.
    world.add_resource(shepherd);
}
