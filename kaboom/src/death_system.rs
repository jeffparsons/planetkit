use specs;
use specs::WriteStorage;
use slog::Logger;

use pk::cell_dweller::CellDweller;
use pk::globe::Globe;

use ::health::Health;

/// Identifies fighters that have run out of health,
/// awards points to their killer, and respawns the victim.
pub struct DeathSystem {
    log: Logger,
}

impl DeathSystem {
    pub fn new(
        parent_log: &Logger,
    ) -> DeathSystem {
        DeathSystem {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for DeathSystem {
    type SystemData = (
        WriteStorage<'a, Health>,
        WriteStorage<'a, CellDweller>,
        WriteStorage<'a, Globe>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;
        use rand::thread_rng;

        let (
            mut healths,
            mut cell_dwellers,
            mut globes,
        ) = data;

        // Find any dead fighters.
        for (cd, health) in (&mut cell_dwellers, &mut healths).join() {
            if health.hp <= 0 {
                // If it was a player that caused them to be harmed,
                // then award a point to that player.
                // TODO: actually track points — and subtract points for a self-kill.
                // TODO: track what player owns this fighter!
                if let Some(last_damaged_by_player_id) = health.last_damaged_by_player_id {
                    // TODO: state the name instead
                    info!(self.log, "Fighter killed!"; "killer_player_id" => format!("{}", last_damaged_by_player_id.0));
                } else {
                    info!(self.log, "Fighter killed by environment!");
                }

                // Re-spawn the player.
                // TODO: actually recreate the entity, rather than doing this.
                // Otherwise we need to find all the components that need
                // to be reset, and that's lame.
                health.hp = 100;

                // Get the associated globe, complaining loudly if we fail.
                // TODO: this same old pattern again.
                let globe_entity = match cd.globe_entity {
                    Some(globe_entity) => globe_entity,
                    None => {
                        warn!(
                            self.log,
                            "There was no associated globe entity or it wasn't actually a Globe! Can't proceed!"
                        );
                        continue;
                    }
                };
                let globe = match globes.get_mut(globe_entity) {
                    Some(globe) => globe,
                    None => {
                        warn!(
                            self.log,
                            "The globe associated with this CellDweller is not alive! Can't proceed!"
                        );
                        continue;
                    }
                };

                // TODO: this is copy-pasted from `fighter.rs`. Factor it out somewhere
                // into a function that knows how to spawn replacement fighters.
                let new_fighter_pos = globe
                    .air_above_random_surface_dry_land(
                        &mut thread_rng(),
                        2, // Min air cells above
                        5, // Max distance from starting point
                        5, // Max attempts
                    )
                    // TODO: don't explode! Just give up and try again on another tick
                    // if it takes too long on this tick! (Leave the player dead for a while;
                    // we'll probably want to do that soon, anyway: make the screen red for
                    // a couple of seconds while they wait to respawn.)
                    .expect(
                        "Oh noes, we took too many attempts to find a decent spawn point!",
                    );

                cd.set_grid_point(new_fighter_pos);
            }
        }
    }
}
