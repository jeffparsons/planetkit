use specs;

use crate::types::*;

/// Velocity relative to some parent entity.
///
/// Assumed to also be a `Spatial`. (That's where its parent
/// reference is stored, and there's no meaning to velocity
/// without position.)
pub struct Velocity {
    local_velocity: Vec3,
}

impl Velocity {
    pub fn new(local_velocity: Vec3) -> Velocity {
        Velocity {
            local_velocity,
        }
    }

    pub fn local_velocity(&self) -> Vec3 {
        self.local_velocity
    }

    pub fn set_local_velocity(&mut self, new_local_velocity: Vec3) {
        self.local_velocity = new_local_velocity;
    }
}

impl<'a> Velocity {
    pub fn local_velocity_mut(&'a mut self) -> &'a mut Vec3 {
        &mut self.local_velocity
    }
}

impl specs::Component for Velocity {
    type Storage = specs::VecStorage<Velocity>;
}
