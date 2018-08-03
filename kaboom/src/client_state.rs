use specs::Entity;

use player::PlayerId;

/// `World`-global resource for client-specific game state.
#[derive(Default)]
pub struct ClientState {
    // This might eventually become a list if, e.g.,
    // we implement multiple players splitscreen on one client.
    pub player_id: Option<PlayerId>,
    // we can unilaterally create the camera entity and
    // never tell other peers about it.
    pub camera_entity: Option<Entity>,
}
