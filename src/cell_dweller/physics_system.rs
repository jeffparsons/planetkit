use specs;
use slog::Logger;

use types::*;
use super::CellDweller;
use ::Spatial;
use globe::Globe;
use globe::chunk::Material;

pub struct PhysicsSystem {
    log: Logger,
    pub seconds_between_falls: TimeDelta,
}

impl PhysicsSystem {
    pub fn new(parent_log: &Logger, seconds_between_falls: TimeDelta) -> PhysicsSystem {
        PhysicsSystem {
            log: parent_log.new(o!()),
            seconds_between_falls: seconds_between_falls,
        }
    }

    // Fall under the force of gravity if there's anywhere to fall to.
    // Note that "gravity" moves you down at a constant speed;
    // i.e. it doesn't accelerate you like in the real world.
    fn maybe_fall(
        &self,
        cd: &mut CellDweller,
        globe: &Globe,
        dt: TimeDelta,
    ) {
        // Only make you fall if there's air below you.
        if cd.pos.z < 0 {
            // There's nothing below; someone built a silly globe.
            return;
        }
        // TODO: this reveals that functions like `set_z`
        // are misleading; this implicitly copies--
        // not changes the orignal!
        let under_pos = cd.pos.set_z(cd.pos.z - 1);
        let under_cell = globe.maybe_non_authoritative_cell(under_pos);
        if under_cell.material == Material::Dirt {
            // Reset time until we can fall to the time
            // between falls; we don't want to instantly
            // fall down every step of size 1.
            cd.seconds_until_next_fall = self.seconds_between_falls;
            return;
        }

        // Count down until we're allowed to fall next.
        if cd.seconds_until_next_fall > 0.0 {
            cd.seconds_until_next_fall = (cd.seconds_until_next_fall - dt).max(0.0);
        }
        let still_waiting_to_fall = cd.seconds_until_next_fall > 0.0;
        if still_waiting_to_fall {
            return;
        }

        // Move down by one cell.
        cd.set_cell_pos(under_pos);
        // REVISIT: += ?
        cd.seconds_until_next_fall = self.seconds_between_falls;
        trace!(self.log, "Fell under force of gravity"; "new_pos" => format!("{:?}", cd.pos()));
    }
}

impl specs::System<TimeDelta> for PhysicsSystem {
    fn run(&mut self, arg: specs::RunArg, dt: TimeDelta) {
        use specs::Join;
        let (mut cell_dwellers, mut spatials, globes) = arg.fetch(|w|
            (w.write::<CellDweller>(), w.write::<Spatial>(), w.read::<Globe>())
        );
        for (cd, spatial) in (&mut cell_dwellers, &mut spatials).iter() {
            // Get the associated globe, complaining loudly if we fail.
            let globe_entity = match cd.globe_entity {
                Some(globe_entity) => globe_entity,
                None => {
                    warn!(self.log, "There was no associated globe entity or it wasn't actually a Globe! Can't proceed!");
                    continue;
                },
            };
            let globe = match globes.get(globe_entity) {
                Some(globe) => globe,
                None => {
                    warn!(self.log, "The globe associated with this CellDweller is not alive! Can't proceed!");
                    continue;
                },
            };

            self.maybe_fall(cd, globe, dt);

            // Update real-space coordinates if necessary.
            // TODO: do this in a separate system; it needs to be done before
            // things are rendered, but there might be other effects like gravity,
            // enemies shunting the cell dweller around, etc. that happen
            // after control.
            if cd.is_real_space_transform_dirty() {
                spatial.transform = cd.get_real_transform_and_mark_as_clean();
            }
        }
    }
}
