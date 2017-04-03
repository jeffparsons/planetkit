use specs;
use pk;
use pk::types::*;

/// Create the player character: a shepherd who must find and rescue the sheep
/// that have strayed from his flock and fallen into holes.
pub fn create_now(world: &mut specs::World, globe_entity: specs::Entity, globe_spec: pk::globe::Spec) -> specs::Entity {
    // TODO: Needing to have access to the ChunkSystem to place the player character
    // is kind of awkward, but might also be correct. The ChunkSystem might need to load
    // chunks from disk to find an appropriate place, so it does seem like we need a reference
    // to it somehow.
    use pk::globe::{ CellPos, Dir };
    let shepherd_pos = CellPos::default();
    world.create_now()
        .with(pk::cell_dweller::CellDweller::new(
            shepherd_pos,
            Dir::default(),
            globe_spec,
            Some(globe_entity),
        ))
        // The CellDweller's transformation will be set based on its coordinates in cell space.
        .with(pk::Spatial::new(globe_entity, Iso3::identity()))
        .build()
}
