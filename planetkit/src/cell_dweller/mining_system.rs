use std::sync::mpsc;
use specs;
use specs::{ WriteStorage, Fetch };
use slog::Logger;
use piston::input::Input;

use super::{ CellDweller, ActiveCellDweller };
use ::movement::*;
use grid::PosInOwningRoot;
use globe::Globe;
use globe::chunk::Material;
use ::input_adapter;

// TODO: own file?
pub struct MiningInputAdapter {
    sender: mpsc::Sender<MiningEvent>,
}

impl MiningInputAdapter {
    pub fn new(sender: mpsc::Sender<MiningEvent>) -> MiningInputAdapter {
        MiningInputAdapter {
            sender: sender,
        }
    }
}

impl input_adapter::InputAdapter for MiningInputAdapter {
    fn handle(&self, input_event: &Input) {
        use piston::input::{ Button, PressEvent, ReleaseEvent };
        use piston::input::keyboard::Key;

        if let Some(Button::Keyboard(key)) = input_event.press_args() {
            if key == Key::U {
                self.sender.send(MiningEvent::PickUp(true)).unwrap();
            }
        }
        if let Some(Button::Keyboard(key)) = input_event.release_args() {
            if key == Key::U {
                self.sender.send(MiningEvent::PickUp(false)).unwrap();
            }
        }
    }
}

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

    // TODO: move into special trait, e.g., `PlanetKitSystem`, and require all systems to be
    // added through my interface. We can then specialise that to automatically call this initialisation
    // code if the system happens to provide it.
    pub fn init(&mut self, world: &mut specs::World) {
        ActiveCellDweller::ensure_registered(world);
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
        use globe::is_point_on_chunk_edge;

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
        let under_pos = cd.pos.with_z(cd.pos.z - 1);
        {
            // Inner scope to fight borrowck.
            let under_cell = globe.maybe_non_authoritative_cell(under_pos);
            if under_cell.material != Material::Dirt {
                return;
            }
        }

        // Ask the globe if there's anything in front of us to "pick up".
        let mut new_pos = cd.pos;
        let mut new_dir = cd.dir;
        move_forward(&mut new_pos, &mut new_dir, globe.spec().root_resolution)
            .expect("CellDweller should have been in good state.");
        let anything_to_pick_up = {
            let cell = globe.maybe_non_authoritative_cell(new_pos);
            cell.material == Material::Dirt
        };
        // Also require that there's air above the block;
        // in my initial use case I don't want to allow mining below
        // the surface.
        let air_above_target = {
            let above_new_pos = new_pos.with_z(new_pos.z + 1);
            let cell = globe.maybe_non_authoritative_cell(above_new_pos);
            cell.material == Material::Air
        };
        let can_pick_up = anything_to_pick_up && air_above_target;
        if can_pick_up {
            // TODO: make a special kind of thing you can pick up.
            // TODO: accept that as a system argument, and have some builders
            // that make it super-easy to configure.
            // The goal here should be that the "block dude" game
            // ends up both concise and legible.
            let new_pos_in_owning_root = PosInOwningRoot::new(
                new_pos,
                globe.spec().root_resolution
            );
            globe.authoritative_cell_mut(
                new_pos_in_owning_root
            ).material = Material::Air;
            // Some extra stuff is only relevant if the cell is on the edge of its chunk.
            if is_point_on_chunk_edge(*new_pos_in_owning_root.pos(), globe.spec().chunk_resolution) {
                // Bump version of owned shared cells.
                globe.increment_chunk_owned_edge_version_for_cell(new_pos_in_owning_root);
                // Propagate change to neighbouring chunks.
                let chunk_origin = globe.origin_of_chunk_owning(new_pos_in_owning_root);
                globe.push_shared_cells_for_chunk(chunk_origin);
            }
            // Mark the view for the containing chunk and those containing each cell surrounding
            // it as being dirty. (This cell might affect the visibility of cells in those chunks.)
            // TODO: different API where you commit to changing a cell
            // in a closure you get back that has a reference to it?
            // Or contains a _wrapper_ around it so it knows if you mutated it? Ooooh.
            globe.mark_chunk_views_affected_by_cell_as_dirty(new_pos);
            // TODO: remember on the cell-dweller that it's carrying something?
            // Or should that be a different kind of component?
            debug!(self.log, "Picked up block"; "pos" => format!("{:?}", new_pos));
        }
    }
}

impl<'a> specs::System<'a> for MiningSystem {
    type SystemData = (
        WriteStorage<'a, CellDweller>,
        WriteStorage<'a, Globe>,
        Fetch<'a, ActiveCellDweller>,
    );

    fn run(&mut self, data: Self::SystemData) {
        self.consume_input();
        let (mut cell_dwellers, mut globes, active_cell_dweller_resource) = data;
        let active_cell_dweller_entity = match active_cell_dweller_resource.maybe_entity {
            Some(entity) => entity,
            None => return,
        };
        let cd = cell_dwellers.get_mut(active_cell_dweller_entity).expect("Someone deleted the controlled entity's CellDweller");

        // Get the associated globe, complaining loudly if we fail.
        let globe_entity = match cd.globe_entity {
            Some(globe_entity) => globe_entity,
            None => {
                warn!(self.log, "There was no associated globe entity or it wasn't actually a Globe! Can't proceed!");
                return;
            },
        };
        let globe = match globes.get_mut(globe_entity) {
            Some(globe) => globe,
            None => {
                warn!(self.log, "The globe associated with this CellDweller is not alive! Can't proceed!");
                return;
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
