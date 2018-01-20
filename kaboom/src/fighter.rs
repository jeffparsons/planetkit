use specs;
use specs::{Fetch, LazyUpdate, Entities};

use pk;
use pk::types::*;
use pk::grid;
use pk::globe::Globe;
use pk::render;
use pk::cell_dweller;
use pk::Health;

/// Create the player character.
pub fn create(
    entities: &Entities,
    updater: &Fetch<LazyUpdate>,
    globe_entity: specs::Entity,
    globe: &mut Globe,
) -> specs::Entity {
    use rand::{XorShiftRng, SeedableRng};

    // Find a suitable spawn point for the player character at the globe surface.
    let (globe_spec, fighter_pos) = {
        let globe_spec = globe.spec();
        // Seed spawn point RNG with world seed.
        let seed = globe_spec.seed;
        let mut rng = XorShiftRng::from_seed([seed, seed, seed, seed]);
        let fighter_pos = globe
            .air_above_random_surface_dry_land(
                &mut rng,
                2, // Min air cells above
                5, // Max distance from starting point
                5, // Max attempts
            )
            .expect(
                "Oh noes, we took too many attempts to find a decent spawn point!",
            );
        (globe_spec, fighter_pos)
    };

    // Make visual appearance of player character.
    // For now this is just an axes mesh.
    let mut fighter_visual = render::Visual::new_empty();
    fighter_visual.proto_mesh = Some(render::make_axes_mesh());

    let entity = entities.create();
    updater.insert(entity, cell_dweller::CellDweller::new(
        fighter_pos,
        grid::Dir::default(),
        globe_spec,
        Some(globe_entity),
    ));
    updater.insert(entity, fighter_visual);
    // The CellDweller's transformation will be set based
    // on its coordinates in cell space.
    updater.insert(entity, pk::Spatial::new(globe_entity, Iso3::identity()));
    // Give the fighter some starting health.
    updater.insert(entity, Health::new(100));
    entity
}
