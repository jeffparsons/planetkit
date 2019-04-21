use piston::input::Input;
use slog::Logger;
use specs;
use specs::{Read, ReadStorage, Write, WriteStorage};
use std::sync::mpsc;

use super::{
    ActiveCellDweller, CellDweller, CellDwellerMessage, SendMessageQueue, TryPickUpBlockMessage,
};
use crate::globe::Globe;
use crate::input_adapter;
use crate::net::{Destination, NetMarker, SendMessage, Transport};

// TODO: own file?
pub struct MiningInputAdapter {
    sender: mpsc::Sender<MiningEvent>,
}

impl MiningInputAdapter {
    pub fn new(sender: mpsc::Sender<MiningEvent>) -> MiningInputAdapter {
        MiningInputAdapter { sender }
    }
}

impl input_adapter::InputAdapter for MiningInputAdapter {
    fn handle(&self, input_event: &Input) {
        use piston::input::keyboard::Key;
        use piston::input::{Button, ButtonState};

        if let Input::Button(button_args) = *input_event {
            if let Button::Keyboard(key) = button_args.button {
                let is_down = match button_args.state {
                    ButtonState::Press => true,
                    ButtonState::Release => false,
                };
                match key {
                    Key::U => self.sender.send(MiningEvent::PickUp(is_down)).unwrap(),
                    _ => (),
                }
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
            input_receiver,
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
}

impl<'a> specs::System<'a> for MiningSystem {
    type SystemData = (
        WriteStorage<'a, CellDweller>,
        WriteStorage<'a, Globe>,
        Read<'a, ActiveCellDweller>,
        Write<'a, SendMessageQueue>,
        ReadStorage<'a, NetMarker>,
    );

    fn run(&mut self, data: Self::SystemData) {
        self.consume_input();

        let (
            mut cell_dwellers,
            mut globes,
            active_cell_dweller_resource,
            mut send_message_queue,
            net_markers,
        ) = data;
        let active_cell_dweller_entity = match active_cell_dweller_resource.maybe_entity {
            Some(entity) => entity,
            None => return,
        };
        let cd = cell_dwellers
            .get_mut(active_cell_dweller_entity)
            .expect("Someone deleted the controlled entity's CellDweller");

        // Get the associated globe, complaining loudly if we fail.
        let globe_entity = match cd.globe_entity {
            Some(globe_entity) => globe_entity,
            None => {
                warn!(
                    self.log,
                    "There was no associated globe entity or it wasn't actually a Globe! Can't proceed!"
                );
                return;
            }
        };
        let globe = match globes.get_mut(globe_entity) {
            Some(globe) => globe,
            None => {
                warn!(
                    self.log,
                    "The globe associated with this CellDweller is not alive! Can't proceed!"
                );
                return;
            }
        };

        // If we're trying to pick up, and from our perspective (we might not be the server)
        // we _can_ pick up, then request to the server to pick up the block.
        if self.pick_up && super::mining::can_pick_up(cd, globe) {
            // Post a message to the server (even if that's us)
            // requesting to remove the block.
            debug!(self.log, "Requesting to pick up a block");

            if send_message_queue.has_consumer {
                // If there's a network consumer, then presumably
                // the entity has been given a global ID.
                let cd_entity_id = net_markers
                    .get(active_cell_dweller_entity)
                    .expect("Shouldn't be trying to tell peers about entities that don't have global IDs!")
                    .id;
                send_message_queue.queue.push_back(SendMessage {
                    // Send the request to the master node, including when that's us.
                    destination: Destination::Master,
                    game_message: CellDwellerMessage::TryPickUpBlock(TryPickUpBlockMessage {
                        cd_entity_id,
                    }),
                    transport: Transport::TCP,
                })
            }
        }
    }
}
