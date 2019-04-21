use nphysics3d::object::BodyHandle;
use specs;

/// A rigid body simulated by nphysics.
pub struct RigidBody {
    pub body_handle: BodyHandle,
}

impl RigidBody {
    pub fn new(body_handle: BodyHandle) -> RigidBody {
        RigidBody {
            body_handle,
        }
    }
}

impl specs::Component for RigidBody {
    type Storage = specs::HashMapStorage<RigidBody>;
}
