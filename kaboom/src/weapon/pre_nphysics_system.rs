// TODO: Delete this, and replace with some systems
// and component types that completely embrace nphysics,
// rather than doing part of the work using our legacy
// physics.

use specs;
use specs::{Read, Write, ReadStorage};

use pk::types::*;
use pk::physics::Velocity;
use pk::nphysics::WorldResource;

use super::Grenade;

pub struct PreNphysicsSystem {
}

impl PreNphysicsSystem {
    pub fn new() -> PreNphysicsSystem {
        PreNphysicsSystem {
        }
    }
}

impl<'a> specs::System<'a> for PreNphysicsSystem {
    type SystemData = (
        Read<'a, TimeDeltaResource>,
        Write<'a, WorldResource>,
        ReadStorage<'a, Velocity>,
        ReadStorage<'a, Grenade>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;

        let (
            _dt,
            mut world_resource,
            velocities,
            grenades,
        ) = data;

        let nphysics_world = &mut world_resource.world;

        // Update the velocity of every grenade.
        for (grenade, velocity) in (&grenades, &velocities).join() {
            // NOTE: Using local position; this will only work if everything
            // is parented on the Globe.

            // Update the body's velocity.
            use nphysics3d::math::Velocity;
            let body = nphysics_world.rigid_body_mut(grenade.body_handle)
                .expect("Who deleted the grenade's rigid body?");
            body.set_velocity(Velocity::new(velocity.local_velocity(), ::na::zero()));

            // NOTE: At the moment its position is being calculated
            // totally independently in two places.
            // We want to demonstrate that our physics integration
            // is _really_ working by making it bounce
            // (eventually with a tiny grace period between each bounce,
            // so you can bounce it into a corner and have it only
            // count as a single bounce, etc.).
        }
    }
}
