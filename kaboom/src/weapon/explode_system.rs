use specs;
use specs::{Fetch, Entities, WriteStorage};
use slog::Logger;

use pk::types::*;

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
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;
        let (
            dt,
            entities,
            mut grenades,
        ) = data;

        for (grenade_entity, grenade) in (&*entities, &mut grenades).join() {
            // Count down each grenade's timer, and remove it if
            // it's been alive too long.
            grenade.time_to_live_seconds -= dt.0;
            if grenade.time_to_live_seconds <= 0.0 {
                info!(self.log, "Kaboom!");
                entities.delete(grenade_entity).expect("Wrong entity generation!");
            }
        }
    }
}
