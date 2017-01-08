use super::Vertex;

pub struct ProtoMesh {
    pub vertexes: Vec<Vertex>,
    pub indexes: Vec<u32>,
}

impl ProtoMesh {
    pub fn new(
        vertexes: Vec<Vertex>,
        indexes: Vec<u32>,
    ) -> ProtoMesh {
        ProtoMesh {
            vertexes: vertexes,
            indexes: indexes,
        }
    }
}
