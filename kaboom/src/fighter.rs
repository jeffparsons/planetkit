use specs;
use specs::{Entities, LazyUpdate, Read};

use pk;
use pk::cell_dweller;
use pk::globe::Globe;
use pk::grid;
use pk::render;
use pk::types::*;

use health::Health;
use player::PlayerId;

pub struct Fighter {
    pub player_id: PlayerId,
    pub seconds_between_shots: TimeDelta,
    pub seconds_until_next_shot: TimeDelta,
}

impl Fighter {
    pub fn new(player_id: PlayerId) -> Fighter {
        Fighter {
            player_id: player_id,
            // TODO: accept as parameter
            seconds_between_shots: 0.5,
            seconds_until_next_shot: 0.0,
        }
    }
}

impl specs::Component for Fighter {
    // TODO: more appropriate storage
    type Storage = specs::VecStorage<Fighter>;
}

/// Create the player character.
pub fn create(
    entities: &Entities,
    updater: &Read<LazyUpdate>,
    globe_entity: specs::Entity,
    globe: &mut Globe,
    player_id: PlayerId,
) -> specs::Entity {
    use rand::{SeedableRng, XorShiftRng};

    // Find a suitable spawn point for the player character at the globe surface.
    let (globe_spec, fighter_pos) = {
        let globe_spec = globe.spec();
        // Seed spawn point RNG with world seed.
        let mut rng = XorShiftRng::from_seed(globe_spec.seed_as_u8_array);
        let fighter_pos = globe
            .air_above_random_surface_dry_land(
                &mut rng, 2, // Min air cells above
                5, // Max distance from starting point
                5, // Max attempts
            )
            .expect("Oh noes, we took too many attempts to find a decent spawn point!");
        (globe_spec, fighter_pos)
    };

    // Make visual appearance of player character.
    // For now this is just an axes mesh.
    let mut fighter_visual = render::Visual::new_empty();
    fighter_visual.proto_mesh = Some(render::make_axes_mesh());

    let entity = entities.create();
    updater.insert(
        entity,
        cell_dweller::CellDweller::new(
            fighter_pos,
            grid::Dir::default(),
            globe_spec,
            Some(globe_entity),
        ),
    );
    updater.insert(entity, fighter_visual);
    // The CellDweller's transformation will be set based
    // on its coordinates in cell space.
    updater.insert(entity, pk::Spatial::new(globe_entity, Iso3::identity()));
    // Give the fighter some starting health.
    updater.insert(entity, Health::new(100));
    updater.insert(entity, ::fighter::Fighter::new(player_id));
    entity
}
