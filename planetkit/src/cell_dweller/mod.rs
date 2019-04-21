// It's a private module; allow this.
// (It's just used for grouping implementation code;
// not in any public interface. Maybe one day I'll revisit
// this and make the internal organisation a bit better,
// but I don't want to be bugged about it for now.)
#[allow(clippy::module_inception)]
mod cell_dweller;
mod mining;
mod mining_system;
mod movement_system;
mod physics_system;
mod recv_system;

use crate::grid::{Dir, Point3};
use crate::movement::TurnDir;
use crate::net::{RecvMessage, SendMessage};
use std::collections::vec_deque::VecDeque;

pub use self::cell_dweller::CellDweller;
pub use self::mining_system::{MiningEvent, MiningInputAdapter, MiningSystem};
pub use self::movement_system::{MovementEvent, MovementInputAdapter, MovementSystem};
pub use self::physics_system::PhysicsSystem;
pub use self::recv_system::RecvSystem;

use specs;

/// `World`-global resource for finding the current cell-dwelling entity being controlled
/// by the player, if any.
///
/// TODO: make this a more general "controlled entity" somewhere?
#[derive(Default)]
pub struct ActiveCellDweller {
    pub maybe_entity: Option<specs::Entity>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum CellDwellerMessage {
    SetPos(SetPosMessage),
    TryPickUpBlock(TryPickUpBlockMessage),
    RemoveBlock(RemoveBlockMessage),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct SetPosMessage {
    pub entity_id: u64,
    pub new_pos: Point3,
    pub new_dir: Dir,
    pub new_last_turn_bias: TurnDir,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct TryPickUpBlockMessage {
    // TODO:
    // pub globe_entity_id: u64,
    pub cd_entity_id: u64,
    // TODO: what are you trying to pick up? Until we hook that up,
    // just use whatever the server thinks is in front of you.
    // pub pos: Point3,
    // TODO: also include the cell dweller's current position.
    // We'll trust that if it's close enough, so that we don't
    // have to worry about missing out on a position update and
    // picking up a different block than what the client believed
    // they were pickng up!
}

// TODO: this shouldn't really even be a cell dweller message;
// it's a more general thing. But it's also not how we want to
// represent the concept of chunk changes long-term, anyway,
// so just leave it in here for now. Hoo boy, lotsa refactoring
// lies ahead.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct RemoveBlockMessage {
    // TODO: identify the globe.
    // But for that, the server will need to communicate the globe's
    // identity etc. to the client when they join.
    // For now it's just going to find the first globe it can... :)
    // pub globe_entity_id: u64,

    // Don't send it as a "PosInOwningRoot", because we can't trust
    // clients like that.
    //
    // TODO: We should actually be validating EVERYTHING that comes
    // in as a network message.
    pub pos: Point3,
}

/// `World`-global resource for outbound cell-dweller network messages.
#[derive(Default)]
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

/// `World`-global resource for inbound cell-dweller network messages.
#[derive(Default)]
pub struct RecvMessageQueue {
    pub queue: VecDeque<RecvMessage<CellDwellerMessage>>,
}
