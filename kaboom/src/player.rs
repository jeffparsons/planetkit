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
    pub peer_id: PeerId,
    pub fighter_entity: Option<specs::Entity>,
}


#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum PlayerMessage {
    NewPlayer(PlayerId),
    // Tell a client about the new player ID created for them,
    // or the player they are taking over.
    YourPlayer(PlayerId),
    NewFighter(u64),
    YourFighter(u64),
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
