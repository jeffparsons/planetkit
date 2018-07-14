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

#[derive(SystemData)]
pub struct PhysicsSystemData<'a> {
    dt: Read<'a, TimeDeltaResource>,
    world_resource: Write<'a, WorldResource>,
    velocities: WriteStorage<'a, Velocity>,
    spatials: WriteStorage<'a, Spatial>,
    rigid_bodies: ReadStorage<'a, RigidBody>,
    remove_body_queue: Write<'a, super::RemoveBodyQueue>,
}

impl<'a> specs::System<'a> for PhysicsSystem {
    type SystemData = PhysicsSystemData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        use specs::Join;

        // NOTE: Everything here is currently using local positions;
        // this will only work if everything is parented on the same Globe.
        // TODO: Move to separate nphysics worlds per "active" Globe.
        // (That's probably a while away, though.)

        let nphysics_world = &mut data.world_resource.world;

        // Remove any bodies that have had their corresponding
        // components in Specs land removed.
        //
        // Note that we might be removing them before the
        // components have actually been cleaned up, so when we
        // look up bodies below, we need to ignore any we don't find.
        while let Some(message) = data.remove_body_queue.queue.pop_front() {
            nphysics_world.remove_bodies(&[message.handle]);
        }

        // Copy all rigid body velocities into the nphysics world.
        for (velocity, rigid_body) in (&data.velocities, &data.rigid_bodies).join() {
            use nphysics3d::math::Velocity;
            // Component might not have been cleaned up, even if we've
            // already deleted the corresponding nphysics body.
            if let Some(body) = nphysics_world.rigid_body_mut(rigid_body.body_handle) {
                body.set_velocity(Velocity::new(velocity.local_velocity(), ::na::zero()));
            }
        }

        // Step the `nphysics` world.
        nphysics_world.set_timestep(data.dt.0);
        nphysics_world.step();

        // Copy position and velocity back out into the Specs world.
        for (rigid_body, spatial, velocity) in (&data.rigid_bodies, &mut data.spatials, &mut data.velocities).join() {
            // Component might not have been cleaned up, even if we've
            // already deleted the corresponding nphysics body.
            if let Some(body) = nphysics_world.rigid_body(rigid_body.body_handle) {
                spatial.set_local_transform(body.position());
                velocity.set_local_velocity(body.velocity().linear);
            }
        }
    }
}
