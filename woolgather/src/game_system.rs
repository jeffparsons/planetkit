use pk;
use specs;
use slog::Logger;

use pk::types::*;
use super::game_state::{ GameState, LevelOutcome };

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

impl pk::System<TimeDelta> for GameSystem {
    fn init(&mut self, world: &mut specs::World) {
        GameState::ensure_registered(world);
    }
}

impl specs::System<TimeDelta> for GameSystem {
    fn run(&mut self, arg: specs::RunArg, _dt: TimeDelta) {
        let (mut game_state,) = arg.fetch(|w|
            (
                w.write_resource::<GameState>(),
            )
        );
        // TEMP: instant-win!
        match game_state.current_level.level_outcome {
            LevelOutcome::Pending => {
                game_state.current_level.level_outcome = LevelOutcome::Won;
                info!(self.logger, "You successfully completed the current level!");
            },
            LevelOutcome::Won => {
                // Nothing can stop us now; we've already won!
            },
            LevelOutcome::_Lost => {
                // Nothing can save us now; we've already lost!
                // TODO: maybe reset the level so we can try again?
            },
        }
    }
}
