mod cell_dweller;
mod movement_system;
mod mining_system;
mod physics_system;

pub use self::cell_dweller::{ CellDweller };
pub use self::movement_system::{ MovementSystem, MovementEvent, MovementInputAdapter };
pub use self::mining_system::{ MiningSystem, MiningEvent, MiningInputAdapter };
pub use self::physics_system::PhysicsSystem;

use shred;
use specs;

/// `World`-global resource for finding the current cell-dwelling entity being controlled
/// by the player, if any.
///
/// TODO: make this a more general "controlled entity" somewhere?
pub struct ActiveCellDweller {
    pub maybe_entity: Option<specs::Entity>,
}

impl ActiveCellDweller {
    pub fn ensure_registered(world: &mut specs::World) {
        let res_id = shred::ResourceId::new::<ActiveCellDweller>();
        if !world.res.has_value(res_id) {
            world.add_resource(ActiveCellDweller {
                maybe_entity: None,
            });
        }
    }
}
