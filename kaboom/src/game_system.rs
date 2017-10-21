use specs;
use specs::FetchMut;
use slog::Logger;

use super::game_state::GameState;

/// System to drive the top-level state machine for level and game state.
pub struct GameSystem {
    _logger: Logger,
}

impl GameSystem {
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> GameSystem {
        use pk::AutoResource;

        // Ensure GameState resource is present.
        GameState::ensure(world);

        GameSystem {
            _logger: parent_log.new(o!("system" => "game"))
        }
    }
}

impl<'a> specs::System<'a> for GameSystem {
    type SystemData = (FetchMut<'a, GameState>,);

    fn run(&mut self, data: Self::SystemData) {
        let (mut _game_state,) = data;

        // ...
    }
}
