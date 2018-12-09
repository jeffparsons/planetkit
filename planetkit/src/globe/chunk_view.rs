use specs;

use crate::globe::ChunkOrigin;

pub struct ChunkView {
    pub globe_entity: specs::Entity,
    pub origin: ChunkOrigin,
}

impl ChunkView {
    pub fn new(globe_entity: specs::Entity, origin: ChunkOrigin) -> ChunkView {
        ChunkView {
            origin: origin,
            globe_entity: globe_entity,
        }
    }
}

impl specs::Component for ChunkView {
    type Storage = specs::HashMapStorage<ChunkView>;
}
