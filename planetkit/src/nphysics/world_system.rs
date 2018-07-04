use specs;
use specs::{Read, Write};

use types::*;
use super::WorldResource;

/// Steps the `nphysics` World.
pub struct WorldSystem {
}

impl WorldSystem {
    pub fn new() -> WorldSystem {
        WorldSystem {
        }
    }
}

impl<'a> specs::System<'a> for WorldSystem {
    type SystemData = (
        Read<'a, TimeDeltaResource>,
        Write<'a, WorldResource>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            dt,
            mut world_resource,
        ) = data;

        let nphysics_world = &mut world_resource.world;

        // Step the `nphysics` world.
        nphysics_world.set_timestep(dt.0);
        nphysics_world.step();
    }
}
