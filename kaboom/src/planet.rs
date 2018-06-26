use specs;
use specs::{Read, LazyUpdate, Entities};

use pk;
use pk::globe::{Globe, Spec};

// Create a planet to fight on.
pub fn create(
    entities: &Entities,
    updater: &Read<LazyUpdate>,
) -> specs::Entity {
    // Make it small enough that you can find another person easily enough.
    // TODO: eventually make it scale to the number of players present at the start of each round.
    // TODO: special generator for this; you want to have lava beneath the land
    let ocean_radius = 30.0;
    let crust_depth = 25.0;
    let floor_radius = ocean_radius - crust_depth;
    let spec = Spec {
        // TODO: random seed every time.
        seed: 14,
        floor_radius: floor_radius,
        ocean_radius: ocean_radius,
        block_height: 0.65,
        // TODO: calculate this (experimentally if necessary) based on the size of the blocks you want
        root_resolution: [64, 128],
        // Chunks should probably be taller, but short chunks are a bit
        // better for now in exposing bugs visually.
        chunk_resolution: [16, 16, 4],
    };
    let globe = Globe::new(spec);

    let entity = entities.create();
    updater.insert(entity, globe);
    updater.insert(entity, pk::Spatial::new_root());
    entity
}
