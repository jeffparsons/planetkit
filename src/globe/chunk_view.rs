use specs;

use globe::CellPos;

pub struct ChunkView {
    pub globe_entity: specs::Entity,
    pub origin: CellPos,
}

impl ChunkView {
    pub fn new(
        globe_entity: specs::Entity,
        origin: CellPos
    ) -> ChunkView {
        ChunkView {
            origin: origin,
            globe_entity: globe_entity,
        }
    }
}

impl specs::Component for ChunkView {
    type Storage = specs::HashMapStorage<ChunkView>;
}
