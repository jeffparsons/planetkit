use specs;

use super::MeshHandle;

pub struct Visual {
    // Even if a component has visual nature, its mesh might
    // not have been created yet at the time the entity is created,
    // and we don't want to have to hold up the show to wait for that.
    // We may also want to change its appearance dynamically.
    mesh_handle: Option<MeshHandle>,
}

impl Visual {
    pub fn new_empty() -> Visual {
        Visual {
            mesh_handle: None,
        }
    }

    pub fn mesh_handle(&self) -> Option<MeshHandle> {
        self.mesh_handle
    }

    pub fn set_mesh_handle(&mut self, new_mesh_handle: MeshHandle) {
        self.mesh_handle = new_mesh_handle.into();
    }
}

impl specs::Component for Visual {
    type Storage = specs::VecStorage<Visual>;
}
