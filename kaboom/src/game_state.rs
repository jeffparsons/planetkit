use specs;

use pk::AutoResource;

/// `World`-global resource for game state.
pub struct GameState {}

impl AutoResource for GameState {
    fn new(_world: &mut specs::World) -> GameState {
        GameState {}
    }
}
