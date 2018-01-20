use specs;
use specs::{Fetch, Entities, ReadStorage, WriteStorage};
use slog::Logger;

use pk::types::*;
use pk::Spatial;

use ::health::Health;
use super::grenade::Grenade;

pub struct ExplodeSystem {
    log: Logger,
}

impl ExplodeSystem {
    pub fn new(
        parent_log: &Logger,
    ) -> ExplodeSystem {
        ExplodeSystem {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for ExplodeSystem {
    type SystemData = (
        Fetch<'a, TimeDeltaResource>,
        Entities<'a>,
        WriteStorage<'a, Grenade>,
        WriteStorage<'a, Health>,
        ReadStorage<'a, Spatial>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;
        use pk::SpatialStorage;

        let (
            dt,
            entities,
            mut grenades,
            mut healths,
            spatials,
        ) = data;

        for (grenade_entity, grenade) in (&*entities, &mut grenades).join() {
            // Count down each grenade's timer, and remove it if
            // it's been alive too long.
            grenade.time_to_live_seconds -= dt.0;
            if grenade.time_to_live_seconds <= 0.0 {
                info!(self.log, "Kaboom!");
                entities.delete(grenade_entity).expect("Wrong entity generation!");

                // Damage anything nearby that can take damage.
                for (living_thing_entity, health, _spatial) in (&*entities, &mut healths, &spatials).join() {
                    let relative_transform = spatials.a_relative_to_b(living_thing_entity, grenade_entity);
                    let distance_squared = relative_transform.translation.vector.norm_squared();
                    let blast_radius_squared = 2.5 * 2.5;

                    if distance_squared <= blast_radius_squared {
                        health.hp -= 100;
                        health.last_damaged_by_player_id = Some(grenade.fired_by_player_id);
                        debug!(self.log, "Damaged something!");
                    }
                }
            }
        }
    }
}
