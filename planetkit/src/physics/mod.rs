// NOTE: a lot of this is going to end up getting
// replaced by nphysics. But it can't hurt to have
// some degenerate versions here for now, to faciliate
// building higher-level bits and pieces.

mod velocity;
mod velocity_system;
mod mass;
mod gravity_system;
mod physics_system;
mod collider;
mod rigid_body;

pub use self::velocity::Velocity;
pub use self::velocity_system::VelocitySystem;
pub use self::mass::Mass;
pub use self::gravity_system::GravitySystem;
pub use self::physics_system::PhysicsSystem;
pub use self::collider::Collider;
pub use self::rigid_body::RigidBody;

use std::collections::vec_deque::VecDeque;
use nphysics3d::world::World;
use nphysics3d::object::{BodyHandle, ColliderHandle};

use types::*;

/// `World`-global resource for nphysics `World`.
pub struct WorldResource {
    pub world: World<Real>,
}

impl Default for WorldResource {
    fn default() -> WorldResource {
        WorldResource {
            world: World::new(),
        }
    }
}

// Work-around for not being able to access removed components
// in Specs FlaggedStorage. This requires systems that remove
// any RigidBody (etc.) components, directly or indirectly by
// deleting the entity, to push an event into this channel.
//
// See <https://github.com/slide-rs/specs/issues/361>.

pub struct RemoveBodyMessage {
    pub handle: BodyHandle,
}

#[derive(Default)]
pub struct RemoveBodyQueue {
    pub queue: VecDeque<RemoveBodyMessage>,
}

pub struct RemoveColliderMessage {
    pub handle: ColliderHandle,
}

#[derive(Default)]
pub struct RemoveColliderQueue {
    pub queue: VecDeque<RemoveColliderMessage>,
}
