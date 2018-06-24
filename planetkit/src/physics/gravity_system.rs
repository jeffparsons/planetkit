use specs;
use specs::{ReadStorage, WriteStorage, Read, Entities};
use slog::Logger;

use types::*;
use super::Velocity;
use super::Mass;
use Spatial;
use globe::Globe;

/// Accelerates everything with mass toward the first globe we find.
// (TODO: this is horrible hacks, but works for Kaboom.)
pub struct GravitySystem {
    _log: Logger,
}

impl GravitySystem {
    pub fn new(parent_log: &Logger) -> GravitySystem {
        GravitySystem {
            _log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for GravitySystem {
    type SystemData = (
        Read<'a, TimeDeltaResource>,
        Entities<'a>,
        ReadStorage<'a, Spatial>,
        WriteStorage<'a, Velocity>,
        ReadStorage<'a, Mass>,
        ReadStorage<'a, Globe>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;
        use spatial::SpatialStorage;

        let (
            dt,
            entities,
            spatials,
            mut velocities,
            masses,
            globes,
        ) = data;

        // For now just find the first globe, and assume that's
        // the one we're supposed to be accelerating towards.
        let globe_entity = match (&*entities, &spatials, &globes).join().next() {
            Some((globe_entity, _spatial, _globe)) => globe_entity,
            // If there's no globe yet, then just do nothing.
            None => return,
        };

        for (mass_entity, _mass, velocity) in (&*entities, &masses, &mut velocities).join() {
            // Accelerate toward the globe. Might as well use Earth gravity for now.
            // Do it "backwards" because we need to strip off the mass's orientation.
            //
            let mass_from_globe = spatials.a_relative_to_b(mass_entity, globe_entity).translation.vector;
            let gravity_direction = -mass_from_globe.normalize();
            let acceleration = gravity_direction * 9.8 * dt.0;
            *velocity.local_velocity_mut() += acceleration;
        }
    }
}
