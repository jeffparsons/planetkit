use shred;
use specs;

/// `World`-global resource for game, including any global state relating
/// to the current level (start time, did you win, etc.) but also any global state
/// that must persist between levels (what campaign is loaded, etc.).
pub struct GameState {
    pub current_level: LevelState,
}

impl GameState {
    pub fn ensure_registered(world: &mut specs::World) {
        let res_id = shred::ResourceId::new::<GameState>();
        if !world.res.has_value(res_id) {
            world.add_resource(GameState::new());
        }
    }

    pub fn new() -> GameState {
        GameState {
            current_level: LevelState::new(),
        }
    }
}

pub struct LevelState {
    pub level_outcome: LevelOutcome,
}

impl LevelState {
    pub fn new() -> LevelState {
        LevelState {
            level_outcome: LevelOutcome::Pending,
        }
    }
}

pub enum LevelOutcome {
    Pending,
    Won,
    _Lost,
}
