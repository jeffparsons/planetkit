use specs;
use specs::{Builder, Entities, LazyUpdate, Read};

use crate::camera::DefaultCamera;
use crate::cell_dweller;
use crate::globe;
use crate::render;
use crate::types::*;

// TODO: Retire most of this stuff. Or, rather, turn it into
// a module (eventually split out into a separate crate) of easy-to-combine
// bits and pieces, but still encouraging best practices.
//
// For example, pretty much all this stuff should be done inside
// a system rather than executing outside of the normal game loop.
// (So that it can interact with other systems, like networking,
// load/save, etc.)

pub fn populate_world(world: &mut specs::World) {
    let globe_entity = create_simple_globe_now(world);
    let player_character_entity = create_simple_player_character_now(world, globe_entity);
    create_simple_chase_camera_now(world, player_character_entity);
}

pub fn create_simple_globe_now(world: &mut specs::World) -> specs::Entity {
    let globe = globe::Globe::new_earth_scale_example();
    world
        .create_entity()
        .with(globe)
        .with(crate::Spatial::new_root())
        .build()
}

pub fn create_simple_player_character_now(
    world: &mut specs::World,
    globe_entity: specs::Entity,
) -> specs::Entity {
    use rand::{SeedableRng, XorShiftRng};

    // Find a suitable spawn point for the player character at the globe surface.
    use crate::grid::Dir;
    let (globe_spec, player_character_pos) = {
        let mut globe_storage = world.write_storage::<globe::Globe>();
        let globe = globe_storage
            .get_mut(globe_entity)
            .expect("Uh oh, it looks like our Globe went missing.");
        let globe_spec = globe.spec();
        // Seed spawn point RNG with world seed.
        let mut rng = XorShiftRng::from_seed(globe_spec.seed_as_u8_array);
        let player_character_pos = globe
            .air_above_random_surface_dry_land(
                &mut rng, 2, // Min air cells above
                5, // Max distance from starting point
                5, // Max attempts
            )
            .expect("Oh noes, we took too many attempts to find a decent spawn point!");
        (globe_spec, player_character_pos)
    };

    // Make visual appearance of player character.
    // For now this is just an axes mesh.
    let mut player_character_visual = render::Visual::new_empty();
    player_character_visual.proto_mesh = Some(render::make_axes_mesh());

    let player_character_entity = world.create_entity()
        .with(cell_dweller::CellDweller::new(
            player_character_pos,
            Dir::default(),
            globe_spec,
            Some(globe_entity),
        ))
        .with(player_character_visual)
        // The CellDweller's transformation will be set based
        // on its coordinates in cell space.
        .with(crate::Spatial::new(globe_entity, Iso3::identity()))
        .build();
    // Set our new character as the currently controlled cell dweller.
    world
        .write_resource::<cell_dweller::ActiveCellDweller>()
        .maybe_entity = Some(player_character_entity);
    player_character_entity
}

pub fn create_simple_chase_camera_now(
    world: &mut specs::World,
    player_character_entity: specs::Entity,
) -> specs::Entity {
    // Create a camera sitting a little bit behind the cell dweller.
    let eye = Pt3::new(0.0, 4.0, -6.0);
    let target = Pt3::origin();
    let camera_transform = Iso3::new_observer_frame(&eye, &target, &Vec3::z());
    let camera_entity = world
        .create_entity()
        .with(crate::Spatial::new(player_character_entity, camera_transform))
        .build();
    use crate::camera::DefaultCamera;
    // TODO: gah, where does this belong?
    world.add_resource(DefaultCamera {
        camera_entity: Some(camera_entity),
    });
    camera_entity
}

pub fn create_simple_chase_camera(
    entities: &Entities,
    updater: &Read<LazyUpdate>,
    player_character_entity: specs::Entity,
    default_camera: &mut DefaultCamera,
) -> specs::Entity {
    // Create a camera sitting a little bit behind the cell dweller.
    let eye = Pt3::new(0.0, 4.0, -6.0);
    let target = Pt3::origin();
    let camera_transform = Iso3::new_observer_frame(&eye, &target, &Vec3::z());
    let entity = entities.create();
    updater.insert(
        entity,
        crate::Spatial::new(player_character_entity, camera_transform),
    );
    default_camera.camera_entity = Some(entity);
    entity
}
