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
        //
        // TODO: also require that there's air above the block;
        // in my initial use case I don't want to allow mining below
        // the surface.
        let can_pick_up = {
            let cell = globe.cell(new_pos);
            cell.material == Material::Dirt
        };
        if can_pick_up {
            // TODO: make a special kind of thing you can pick up.
            // TODO: accept that as a system argument, and have some builders
            // that make it super-easy to configure.
            // The goal here should be that the "block dude" game
            // ends up both concise and legible.
            globe.cell_mut(new_pos).material = Material::Air;
            // Mark the containing chunkas being dirty.
            // TODO: different API where you commit to changing a cell
            // in a closure you get back that has a reference to it?
            // Or contains a _wrapper_ around it so it knows if you mutated it? Ooooh.
            globe.mark_chunk_view_as_dirty(new_pos);
            // Propagate change to neighbouring chunks.
            // TODO: we reaaaally need a better interface for mutating chunk data
            // to make sure this happens automatically.
            //
            // TODO: also this is _stupidly_ slow hacks!
            // Like... _really_ bad. Fixing the interface to cell data
            // is now a high priority.
            globe.copy_all_authoritative_cells();
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
        }
    }
}
