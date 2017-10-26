mod cell_dweller;
mod movement_system;
mod mining_system;
mod physics_system;
mod recv_system;

use std::collections::vec_deque::VecDeque;
use grid::{GridPoint3, Dir};
use ::movement::TurnDir;
use ::net::{SendMessage, RecvMessage};

pub use ::AutoResource;
pub use self::cell_dweller::CellDweller;
pub use self::movement_system::{MovementSystem, MovementEvent, MovementInputAdapter};
pub use self::mining_system::{MiningSystem, MiningEvent, MiningInputAdapter};
pub use self::physics_system::PhysicsSystem;
pub use self::recv_system::RecvSystem;

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
    // TODO: replace with AutoResource
    pub fn ensure_registered(world: &mut specs::World) {
        let res_id = shred::ResourceId::new::<ActiveCellDweller>();
        if !world.res.has_value(res_id) {
            world.add_resource(ActiveCellDweller { maybe_entity: None });
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum CellDwellerMessage {
    SetPos(SetPosMessage),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct SetPosMessage {
    pub entity_id: u64,
    pub new_pos: GridPoint3,
    pub new_dir: Dir,
    pub new_last_turn_bias: TurnDir,
}

/// `World`-global resource for outbound cell-dweller network messages.
pub struct SendMessageQueue {
    // We don't want to queue up any messages unless there's
    // actually a network system hanging around to consume them.
    // TODO: there's got to be a better way to do this.
    // I'm thinking some kind of simple pubsub, that doesn't
    // know anything about atomics/thread synchronisation,
    // but is instead just a dumb collection of `VecDeque`s.
    // As you add more of these, either find something that works
    // or make that thing you described above.
    pub has_consumer: bool,
    pub queue: VecDeque<SendMessage<CellDwellerMessage>>,
}

impl ::AutoResource for SendMessageQueue {
    fn new(_world: &mut specs::World) -> SendMessageQueue {
        SendMessageQueue {
            has_consumer: false,
            queue: VecDeque::new(),
        }
    }
}

/// `World`-global resource for inbound cell-dweller network messages.
pub struct RecvMessageQueue {
    pub queue: VecDeque<RecvMessage<CellDwellerMessage>>,
}

impl ::AutoResource for RecvMessageQueue {
    fn new(_world: &mut specs::World) -> RecvMessageQueue {
        RecvMessageQueue {
            queue: VecDeque::new(),
        }
    }
}
