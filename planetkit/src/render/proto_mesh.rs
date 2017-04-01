use super::Vertex;

#[derive(Clone)]
pub struct ProtoMesh {
    pub vertexes: Vec<Vertex>,
    pub indexes: Vec<u32>,
}

impl ProtoMesh {
    /// Panicks if given an empty vertex or index vector.
    pub fn new(
        vertexes: Vec<Vertex>,
        indexes: Vec<u32>,
    ) -> ProtoMesh {
        // Don't allow creating empty mesh.
        // Back-end doesn't seem to like this, and it probably represents
        // a mistake if we attempt this anyway.
        assert!(vertexes.len() > 0);
        assert!(indexes.len() > 0);

        ProtoMesh {
            vertexes: vertexes,
            indexes: indexes,
        }
    }
}
