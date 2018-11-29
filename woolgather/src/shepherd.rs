use crate::pk;
use crate::pk::cell_dweller;
use crate::pk::globe;
use crate::pk::grid;
use crate::pk::render;
use crate::pk::types::*;
use specs::{self, Builder};

/// Create the player character: a shepherd who must find and rescue the sheep
/// that have strayed from his flock and fallen into holes.
pub fn create_now(world: &mut specs::World, globe_entity: specs::Entity) -> specs::Entity {
    use rand::{SeedableRng, XorShiftRng};

    // Find a suitable spawn point for the player character at the globe surface.
    let (globe_spec, shepherd_pos) = {
        let mut globe_storage = world.write_storage::<globe::Globe>();
        let globe = globe_storage
            .get_mut(globe_entity)
            .expect("Uh oh, it looks like our Globe went missing.");
        let globe_spec = globe.spec();
        // Seed spawn point RNG with world seed.
        let mut rng = XorShiftRng::from_seed(globe_spec.seed_as_u8_array);
        let shepherd_pos = globe
            .air_above_random_surface_dry_land(
                &mut rng, 2, // Min air cells above
                5, // Max distance from starting point
                5, // Max attempts
            )
            .expect("Oh noes, we took too many attempts to find a decent spawn point!");
        (globe_spec, shepherd_pos)
    };

    // Make visual appearance of player character.
    // For now this is just an axes mesh.
    let mut shepherd_visual = render::Visual::new_empty();
    shepherd_visual.proto_mesh = Some(render::make_axes_mesh());

    let shepherd_entity = world.create_entity()
        .with(cell_dweller::CellDweller::new(
            shepherd_pos,
            grid::Dir::default(),
            globe_spec,
            Some(globe_entity),
        ))
        .with(shepherd_visual)
        // The CellDweller's transformation will be set based
        // on its coordinates in cell space.
        .with(pk::Spatial::new(globe_entity, Iso3::identity()))
        .build();
    shepherd_entity
}
