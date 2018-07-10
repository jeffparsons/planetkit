use specs;
use nphysics3d::object::{BodyHandle, ColliderHandle};

// use pk::types::*;

/// A rigid body simulated by nphysics.
pub struct RigidBody {
    pub body_handle: BodyHandle,
    pub collider_handle: ColliderHandle,
}

impl RigidBody {
    pub fn new(body_handle: BodyHandle, collider_handle: ColliderHandle) -> RigidBody {
        RigidBody {
            collider_handle: collider_handle,
            body_handle: body_handle,
        }
    }
}

impl specs::Component for RigidBody {
    type Storage = specs::HashMapStorage<RigidBody>;
}
