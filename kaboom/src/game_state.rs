use std::collections::vec_deque::VecDeque;

use specs::{self, Entity};

use pk::AutoResource;

use ::player::{Player, PlayerId};

/// `World`-global resource for game state,
/// largely defined by the server, but much also maintained
/// by clients as they are informed about the state of the world.
pub struct GameState {
    pub globe_entity: Option<Entity>,
    // TODO: this should probably not be a Vec;
    // in practice we can be pretty sure clients will
    // hear about new players in order, but it's still not
    // the right kind of structure to store this in.
    pub players: Vec<Player>,
    // New players that have joined but haven't been initialized.
    // Only the server cares about this.
    // (TODO: split it out into a ServerState or MasterState struct?
    // Maybe that's not worth it...)
    pub new_players: VecDeque<PlayerId>,
}

impl AutoResource for GameState {
    fn new(_world: &mut specs::World) -> GameState {
        GameState {
            globe_entity: None,
            players: Vec::<Player>::new(),
            new_players: VecDeque::<PlayerId>::new(),
        }
    }
}
