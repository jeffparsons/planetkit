use specs;
use specs::{Read, Write, ReadStorage, WriteStorage};

use types::*;
use ::Spatial;
use super::{WorldResource, Velocity, RigidBody};

/// Synchronises state between the Specs and nphysics worlds,
/// and drives the nphysics simulation.
///
/// In order, this system
///
/// 1. Copies state that has been altered in the Specs world
///    (e.g. velocity) into the nphysics world.
/// 2. Steps the nphysics world.
/// 3. Copies state from the nphysics world (e.g. position, orientation)
///    back out into the Specs world.
// TODO: How are we going to be communicating collision events
// into Specs land? Just make everyone who cares iterate over all of them?
// Or make every system register its interest in particular objects?
pub struct PhysicsSystem {
}

impl PhysicsSystem {
    pub fn new() -> PhysicsSystem {
        PhysicsSystem {
        }
    }
}

impl<'a> specs::System<'a> for PhysicsSystem {
    type SystemData = (
        Read<'a, TimeDeltaResource>,
        Write<'a, WorldResource>,
        WriteStorage<'a, Velocity>,
        WriteStorage<'a, Spatial>,
        ReadStorage<'a, RigidBody>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;

        // NOTE: Everything here is currently using local positions;
        // this will only work if everything is parented on the same Globe.
        // TODO: Move to separate nphysics worlds per "active" Globe.
        // (That's probably a while away, though.)

        let (
            dt,
            mut world_resource,
            mut velocities,
            mut spatials,
            rigid_bodies,
        ) = data;

        let nphysics_world = &mut world_resource.world;

        // Copy all rigid body velocities into the nphysics world.
        for (velocity, rigid_body) in (&velocities, &rigid_bodies).join() {
            use nphysics3d::math::Velocity;
            let body = nphysics_world.rigid_body_mut(rigid_body.body_handle)
                .expect("Who deleted the rigid body? We weren't done with that!");
            body.set_velocity(Velocity::new(velocity.local_velocity(), ::na::zero()));
        }

        // Step the `nphysics` world.
        nphysics_world.set_timestep(dt.0);
        nphysics_world.step();

        // Copy position and velocity back out into the Specs world.
        for (rigid_body, spatial, velocity) in (&rigid_bodies, &mut spatials, &mut velocities).join() {
            let body = nphysics_world.rigid_body(rigid_body.body_handle)
                .expect("Who deleted the rigid body? We weren't done with that!");
            spatial.set_local_transform(body.position());
            velocity.set_local_velocity(body.velocity().linear);
        }
    }
}
