use specs::{self, Entity};

use pk::AutoResource;

use ::player::PlayerId;

/// `World`-global resource for client-specific game state.
pub struct ClientState {
    // This might eventually become a list if, e.g.,
    // we implement multiple players splitscreen on one client.
    pub player_id: Option<PlayerId>,
    // we can unilaterally create the camera entity and
    // never tell other peers about it.
    pub camera_entity: Option<Entity>,
}

impl AutoResource for ClientState {
    fn new(_world: &mut specs::World) -> ClientState {
        ClientState {
            player_id: None,
            camera_entity: None,
        }
    }
}
