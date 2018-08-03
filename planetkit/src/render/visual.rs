use froggy;
use specs;

use super::MeshWrapper;
use super::ProtoMesh;

pub struct Visual {
    // Even if a component has visual nature, its mesh might
    // not have been created yet at the time the entity is created,
    // and we don't want to have to hold up the show to wait for that.
    // We may also want to change its appearance dynamically.
    mesh_pointer: Option<froggy::Pointer<MeshWrapper>>,
    // Vertex and index data that hasn't yet been sent to
    // the video card. Render system uses this to replace the
    // actual mesh whenever this is present.
    // TODO: privacy
    pub proto_mesh: Option<ProtoMesh>,
}

impl Visual {
    pub fn new_empty() -> Visual {
        Visual {
            mesh_pointer: None,
            proto_mesh: None,
        }
    }

    pub fn mesh_pointer(&self) -> Option<&froggy::Pointer<MeshWrapper>> {
        self.mesh_pointer.as_ref()
    }

    pub fn set_mesh_pointer(&mut self, new_mesh_pointer: froggy::Pointer<MeshWrapper>) {
        self.mesh_pointer = new_mesh_pointer.into();
    }
}

impl specs::Component for Visual {
    type Storage = specs::VecStorage<Visual>;
}
