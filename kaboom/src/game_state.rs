use specs::{self, Entity};

use pk::AutoResource;

/// `World`-global resource for game state.
pub struct GameState {
    // Are we the server/owner of the game?
    pub is_master: bool,
    pub globe_entity: Option<Entity>,
    pub fighter_entity: Option<Entity>,
    pub camera_entity: Option<Entity>,
}

impl AutoResource for GameState {
    fn new(_world: &mut specs::World) -> GameState {
        GameState {
            // This will get set to something meaningful
            // when hosting/joining a game.
            is_master: false,
            globe_entity: None,
            fighter_entity: None,
            camera_entity: None,
        }
    }
}
