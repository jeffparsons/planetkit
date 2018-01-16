use std::collections::vec_deque::VecDeque;

use specs;

use pk::AutoResource;
use pk::net::{PeerId, RecvMessage};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Copy)]
pub struct PlayerId(pub u16);

pub struct Player {
    pub id: PlayerId,
    // There could be many players per network peer,
    // e.g., if we ever get around to adding splitscreen.
    //
    // TODO: does this need to be a local peer ID?
    // I don't see any reason this can't be a global one.
    // E.g. node_id. Then we don't need to do any swizzling when
    // transferring these entities around -- everyone gets a global ID.
    //
    // TODO: definitely revisit this once `specs::saveload` is released (0.11?)
    // and you can start using that.
    pub peer_id: PeerId,
    pub fighter_entity: Option<specs::Entity>,
    pub name: String,
}


#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum PlayerMessage {
    NewPlayer(NewPlayerMessage),
    // Tell a client about the new player ID created for them,
    // or the player they are taking over.
    YourPlayer(PlayerId),
    NewFighter(u64),
    YourFighter(u64),
}

// REVISIT: just serialize an entire player instead,
// once everything in it is global? Only if there's
// no privileged information in it.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct NewPlayerMessage {
    pub id: PlayerId,
    pub name: String,
}

/// `World`-global resource for inbound player-related network messages.
pub struct RecvMessageQueue {
    pub queue: VecDeque<RecvMessage<PlayerMessage>>,
}

impl AutoResource for RecvMessageQueue {
    fn new(_world: &mut specs::World) -> RecvMessageQueue {
        RecvMessageQueue {
            queue: VecDeque::new(),
        }
    }
}
