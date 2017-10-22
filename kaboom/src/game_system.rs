use specs;
use specs::{WriteStorage, Fetch, FetchMut, LazyUpdate, Entities};
use slog::Logger;

use pk;
use pk::globe::Globe;
use pk::cell_dweller::{CellDweller, ActiveCellDweller};
use pk::camera::DefaultCamera;

use ::game_state::GameState;
use ::planet;
use ::fighter;

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
    type SystemData = (
        FetchMut<'a, GameState>,
        Entities<'a>,
        Fetch<'a, LazyUpdate>,
        WriteStorage<'a, Globe>,
        FetchMut<'a, ActiveCellDweller>,
        WriteStorage<'a, CellDweller>,
        FetchMut<'a, DefaultCamera>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut game_state,
            entities,
            updater,
            mut globes,
            mut active_cell_dweller,
            cell_dwellers,
            mut default_camera,
        ) = data;

        if game_state.globe_entity.is_none() {
            // Create the globe first, because we'll need it to figure out where
            // to place the player character.
            game_state.globe_entity = Some(
                planet::create(&entities, &updater)
            );
        }

        // TODO: don't create your own guy unless you're the master.
        // Instead receive updates about players joining,
        // create entities for them, and then tell them about the
        // entity that is their player character.

        // TODO: before that, just fight over the same character. :D

        if game_state.fighter_entity.is_none() {
            if let Some(globe_entity) = game_state.globe_entity {
                // We can only do this after the globe has been realized.
                if let Some(mut globe) = globes.get_mut(globe_entity) {
                    // Create the player character.
                    let fighter_entity = fighter::create(
                        &entities,
                        &updater,
                        globe_entity,
                        &mut globe,
                    );
                    game_state.fighter_entity = Some(fighter_entity);
                    // Set our new player character as the currently controlled cell dweller.
                    active_cell_dweller.maybe_entity = Some(fighter_entity);
                }
            }
        }

        if game_state.camera_entity.is_none() {
            if let Some(fighter_entity) = game_state.fighter_entity {
                // We can only do this after the fighter has been realized.
                // TODO: there's got to be a better pattern for this...
                if let Some(_cell_dweller) = cell_dwellers.get(fighter_entity) {
                    // Create basic third-person following camera.
                    game_state.camera_entity = Some(
                        pk::simple::create_simple_chase_camera(
                            &entities,
                            &updater,
                            fighter_entity,
                            &mut default_camera,
                        )
                    );
                }
            }
        }
    }
}
