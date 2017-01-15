use std::sync::mpsc;
use specs;
use slog::Logger;

use types::*;
use super::CellDweller;
use ::movement::*;
use globe::Globe;
use globe::chunk::Material;

pub enum MiningEvent {
    PickUp(bool),
}

pub struct MiningSystem {
    input_receiver: mpsc::Receiver<MiningEvent>,
    log: Logger,
    // TODO: need a better way to deal with one-off events:
    // just set pick_up to false once we've processed it?
    // But Piston seems to have some kind of silly key-repeat thing built in.
    // TODO: clarify.
    pick_up: bool,
}

impl MiningSystem {
    pub fn new(input_receiver: mpsc::Receiver<MiningEvent>, parent_log: &Logger) -> MiningSystem {
        MiningSystem {
            input_receiver: input_receiver,
            log: parent_log.new(o!()),
            pick_up: false,
        }
    }

    fn consume_input(&mut self) {
        loop {
            match self.input_receiver.try_recv() {
                Ok(MiningEvent::PickUp(b)) => self.pick_up = b,
                Err(_) => return,
            }
        }
    }

    fn pick_up_if_possible(
        &self,
        cd: &mut CellDweller,
        globe: &mut Globe,
    ) {
        // Only allow picking stuff up if you're sitting above solid ground.
        // (Or, rather, the stuff we consider to be solid for now,
        // which is anything other than air.)
        //
        // TODO: abstract this whole thing... you need some kind of
        // utilities for a globe.
        if cd.pos.z < 0 {
            // There's nothing below; someone built a silly globe.
            return;
        }
        // TODO: this reveals that functions like `set_z`
        // are misleading; this implicitly copies (because it consumes self)--
        // not changes the orignal!
        let under_pos = cd.pos.set_z(cd.pos.z - 1);
        {
            // Inner scope to fight borrowck.
            let under_cell = globe.cell(under_pos);
            if under_cell.material != Material::Dirt {
                return;
            }
        }

        // Find out whether there's anything in front of us to "pick up".
        let mut new_pos = cd.pos;
        let mut new_dir = cd.dir;
        move_forward(&mut new_pos, &mut new_dir, globe.spec().root_resolution)
            .expect("CellDweller should have been in good state.");
        // Ask the globe if there's anything to pick up.
        let mut cell = globe.cell_mut(new_pos);
        // TODO: make a special kind of thing you can pick up.
        // TODO: accept that as a system argument, and have some builders
        // that make it super-easy to configure.
        // The goal here should be that the "block dude" game
        // ends up both concise and legible.
        let can_pick_up = cell.material == Material::Dirt;
        if can_pick_up {
            cell.material = Material::Air;
            // TODO: remember on the cell-dweller that it's carrying something?
            // Or should that be a different kind of component?
            debug!(self.log, "Picked up block"; "pos" => format!("{:?}", new_pos));
        }
    }
}

impl specs::System<TimeDelta> for MiningSystem {
    fn run(&mut self, arg: specs::RunArg, _dt: TimeDelta) {
        use specs::Join;
        self.consume_input();
        let (mut cell_dwellers, mut globes) = arg.fetch(|w|
            (w.write::<CellDweller>(), w.write::<Globe>())
        );
        for cd in (&mut cell_dwellers).iter() {
            // Get the associated globe, complaining loudly if we fail.
            let globe_entity = match cd.globe_entity {
                Some(globe_entity) => globe_entity,
                None => {
                    warn!(self.log, "There was no associated globe entity or it wasn't actually a Globe! Can't proceed!");
                    continue;
                },
            };
            let globe = match globes.get_mut(globe_entity) {
                Some(globe) => globe,
                None => {
                    warn!(self.log, "The globe associated with this CellDweller is not alive! Can't proceed!");
                    continue;
                },
            };

            if self.pick_up {
                self.pick_up_if_possible(
                    cd,
                    globe,
                );
            }

            // TODO: Flag the chunk as dirty so we can re-build its mesh.
        }
    }
}
