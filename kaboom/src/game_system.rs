use specs;
use specs::FetchMut;
use slog::Logger;

use super::game_state::GameState;

/// System to drive the top-level state machine for level and game state.
pub struct GameSystem {
    logger: Logger,
}

impl GameSystem {
    pub fn new(parent_log: &Logger) -> GameSystem {
        GameSystem {
            logger: parent_log.new(o!("system" => "game")),
        }
    }
}

impl<'a> specs::System<'a> for GameSystem {
    type SystemData = (
        FetchMut<'a, GameState>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut game_state,) = data;

        // ...
    }
}
