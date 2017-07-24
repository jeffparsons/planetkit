use specs;
use specs::FetchMut;
use slog::Logger;

use super::game_state::{GameState, LevelOutcome};

/// System to drive the top-level state machine for level and game state.
pub struct GameSystem {
    logger: Logger,
}

impl GameSystem {
    pub fn new(parent_log: &Logger) -> GameSystem {
        GameSystem { logger: parent_log.new(o!("system" => "game")) }
    }
}

impl<'a> specs::System<'a> for GameSystem {
    type SystemData = (FetchMut<'a, GameState>,);

    fn run(&mut self, data: Self::SystemData) {
        let (mut game_state,) = data;
        // TEMP: instant-win!
        match game_state.current_level.level_outcome {
            LevelOutcome::Pending => {
                game_state.current_level.level_outcome = LevelOutcome::Won;
                info!(self.logger, "You successfully completed the current level!");
            }
            LevelOutcome::Won => {
                // Nothing can stop us now; we've already won!
            }
            LevelOutcome::_Lost => {
                // Nothing can save us now; we've already lost!
                // TODO: maybe reset the level so we can try again?
            }
        }
    }
}
