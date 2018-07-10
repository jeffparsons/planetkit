// TODO: Delete this, and replace with some systems
// and component types that completely embrace nphysics,
// rather than doing part of the work using our legacy
// physics.

use specs;
use specs::{Read, ReadStorage, WriteStorage};

use pk::types::*;
use pk::Spatial;
use pk::physics::WorldResource;

use super::Grenade;

pub struct PostNphysicsSystem {
}

impl PostNphysicsSystem {
    pub fn new() -> PostNphysicsSystem {
        PostNphysicsSystem {
        }
    }
}

impl<'a> specs::System<'a> for PostNphysicsSystem {
    type SystemData = (
        Read<'a, TimeDeltaResource>,
        Read<'a, WorldResource>,
        WriteStorage<'a, Spatial>,
        ReadStorage<'a, Grenade>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;

        let (
            _dt,
            world_resource,
            mut spatials,
            grenades,
        ) = data;

        let nphysics_world = &world_resource.world;

        // Update the position of every grenade in _our_ world.
        for (grenade, spatial) in (&grenades, &mut spatials).join() {
            // NOTE: Using local position; this will only work if everything
            // is parented on the Globe.

            // Update the grenade's spatial based on
            // whatever nphysics did to it.
            let body = nphysics_world.rigid_body(grenade.body_handle)
                .expect("Who deleted the grenade's rigid body?");
            spatial.set_local_transform(body.position());

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
