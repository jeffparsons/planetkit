mod physics_system;

pub use self::physics_system::PhysicsSystem;

use nphysics3d::world::World;

use types::*;

/// `World`-global resource for nphysics `World`.
pub struct WorldResource {
    pub world: World<Real>,
}

impl Default for WorldResource {
    fn default() -> WorldResource {
        WorldResource {
            // TODO: remove the ground? I don't think we want it,
            // and it appears to be created by default.
            // Or... there are no colliders by default, so maybe
            // the ground just exists as a "root" object?
            world: World::new(),
        }
    }
}
