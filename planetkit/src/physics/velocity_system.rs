use na;
use specs;
use specs::{ReadStorage, WriteStorage, ReadExpect};
use slog::Logger;

use types::*;
use super::Velocity;
use Spatial;

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
        ReadExpect<'a, TimeDeltaResource>,
        WriteStorage<'a, Spatial>,
        ReadStorage<'a, Velocity>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;
        let (
            dt,
            mut spatials,
            velocities,
        ) = data;
        for (spatial, velocity) in (&mut spatials, &velocities).join() {
            // Apply velocity to spatial.
            let mut local_transform = spatial.local_transform();
            let translation = na::Translation3::<f64>::from_vector(velocity.local_velocity() * dt.0);
            local_transform.append_translation_mut(&translation);
            spatial.set_local_transform(local_transform);
        }
    }
}
