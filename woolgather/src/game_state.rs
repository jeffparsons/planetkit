/// `World`-global resource for game, including any global state relating
/// to the current level (start time, did you win, etc.) but also any global state
/// that must persist between levels (what campaign is loaded, etc.).
#[derive(Default)]
pub struct GameState {
    pub current_level: LevelState,
}

pub struct LevelState {
    // Have we created everything for the level yet?
    pub initialized: bool,
    pub level_outcome: LevelOutcome,
}

impl Default for LevelState {
    fn default() -> LevelState {
        LevelState {
            initialized: false,
            level_outcome: LevelOutcome::Pending,
        }
    }
}

pub enum LevelOutcome {
    Pending,
    Won,
    _Lost,
}
