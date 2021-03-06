use crate::na;
use slog::Logger;
use specs;
use specs::{Read, ReadStorage, WriteStorage};

use super::Velocity;
use crate::types::*;
use crate::Spatial;

pub struct VelocitySystem {
    _log: Logger,
}

impl VelocitySystem {
    pub fn new(parent_log: &Logger) -> VelocitySystem {
        VelocitySystem {
            _log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for VelocitySystem {
    type SystemData = (
        Read<'a, TimeDeltaResource>,
        WriteStorage<'a, Spatial>,
        ReadStorage<'a, Velocity>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;
        let (dt, mut spatials, velocities) = data;
        for (spatial, velocity) in (&mut spatials, &velocities).join() {
            // Apply velocity to spatial.
            let mut local_transform = spatial.local_transform();
            let translation = na::Translation3::<f64>::from(velocity.local_velocity() * dt.0);
            local_transform.append_translation_mut(&translation);
            spatial.set_local_transform(local_transform);
        }
    }
}
