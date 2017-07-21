use shred;
use specs;

/// `World`-global resource for game state.
pub struct GameState {

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
        }
    }
}
