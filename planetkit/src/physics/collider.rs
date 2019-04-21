use nphysics3d::object::ColliderHandle;
use specs;

/// A rigid body simulated by nphysics.
pub struct Collider {
    pub collider_handle: ColliderHandle,
}

impl Collider {
    pub fn new(collider_handle: ColliderHandle) -> Collider {
        Collider { collider_handle }
    }
}

impl specs::Component for Collider {
    type Storage = specs::HashMapStorage<Collider>;
}
