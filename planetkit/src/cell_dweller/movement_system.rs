use std::sync::mpsc;
use specs;
use specs::{ReadStorage, WriteStorage, Fetch, FetchMut};
use slog::Logger;
use piston::input::Input;

use types::*;
use super::{
    CellDweller,
    ActiveCellDweller,
    SendMessageQueue,
    CellDwellerMessage,
    SetPosMessage,
};
use Spatial;
use movement::*;
use globe::Globe;
use globe::chunk::Material;
use input_adapter;
use ::net::{
    SendMessage,
    Transport,
    PeerId,
};

// TODO: own file?
pub struct MovementInputAdapter {
    sender: mpsc::Sender<MovementEvent>,
}

impl MovementInputAdapter {
    pub fn new(sender: mpsc::Sender<MovementEvent>) -> MovementInputAdapter {
        MovementInputAdapter { sender: sender }
    }
}

impl input_adapter::InputAdapter for MovementInputAdapter {
    fn handle(&self, input_event: &Input) {
        use piston::input::{Button, PressEvent, ReleaseEvent};
        use piston::input::keyboard::Key;

        if let Some(Button::Keyboard(key)) = input_event.press_args() {
            match key {
                Key::I => self.sender.send(MovementEvent::StepForward(true)).unwrap(),
                Key::K => self.sender.send(MovementEvent::StepBackward(true)).unwrap(),
                Key::J => self.sender.send(MovementEvent::TurnLeft(true)).unwrap(),
                Key::L => self.sender.send(MovementEvent::TurnRight(true)).unwrap(),
                _ => (),
            }
        }
        if let Some(Button::Keyboard(key)) = input_event.release_args() {
            match key {
                Key::I => self.sender.send(MovementEvent::StepForward(false)).unwrap(),
                Key::K => {
                    self.sender
                        .send(MovementEvent::StepBackward(false))
                        .unwrap()
                }
                Key::J => self.sender.send(MovementEvent::TurnLeft(false)).unwrap(),
                Key::L => self.sender.send(MovementEvent::TurnRight(false)).unwrap(),
                _ => (),
            }
        }
    }
}

pub enum MovementEvent {
    StepForward(bool),
    StepBackward(bool),
    TurnLeft(bool),
    TurnRight(bool),
}

pub struct MovementSystem {
    input_receiver: mpsc::Receiver<MovementEvent>,
    log: Logger,
    step_forward: bool,
    step_backward: bool,
    turn_left: bool,
    turn_right: bool,
    max_step_height: u8,
}

enum ForwardOrBackward {
    Forward,
    Backward,
}

impl MovementSystem {
    pub fn new(
        world: &mut specs::World,
        input_receiver: mpsc::Receiver<MovementEvent>,
        parent_log: &Logger,
    ) -> MovementSystem {
        use ::AutoResource;
        SendMessageQueue::ensure(world);

        MovementSystem {
            input_receiver: input_receiver,
            log: parent_log.new(o!()),
            step_forward: false,
            step_backward: false,
            turn_left: false,
            turn_right: false,
            max_step_height: 1,
        }
    }

    // TODO: move into special trait, e.g., `PlanetKitSystem`, and require all systems to be
    // added through my interface. We can then specialise that to automatically call this initialisation
    // code if the system happens to provide it.
    pub fn init(&mut self, world: &mut specs::World) {
        // TODO: move this into `new`.
        ActiveCellDweller::ensure_registered(world);
    }

    // Pretty much only for tests.
    pub fn set_step_height(&mut self, new_max_step_height: u8) {
        self.max_step_height = new_max_step_height;
    }

    fn consume_input(&mut self) {
        loop {
            match self.input_receiver.try_recv() {
                Ok(MovementEvent::StepForward(b)) => self.step_forward = b,
                Ok(MovementEvent::StepBackward(b)) => self.step_backward = b,
                Ok(MovementEvent::TurnLeft(b)) => self.turn_left = b,
                Ok(MovementEvent::TurnRight(b)) => self.turn_right = b,
                Err(_) => return,
            }
        }
    }

    fn step_if_possible(
        &self,
        cd: &mut CellDweller,
        globe: &Globe,
        forward_or_backward: ForwardOrBackward,
    ) {
        // Only allow movement if you're sitting above solid ground.
        // (Or, rather, the stuff we consider to be solid for now,
        // which is anything other than air.)
        //
        // TODO: Fix to be <= 0 and log error.
        if cd.pos.z < 0 {
            // There's nothing below; someone built a silly globe.
            return;
        }
        let under_pos = cd.pos.with_z(cd.pos.z - 1);
        let under_cell = globe.maybe_non_authoritative_cell(under_pos);
        if under_cell.material != Material::Dirt {
            return;
        }

        // Find out whether we're actually allowed to step there.
        let mut new_pos = cd.pos;
        let mut new_dir = cd.dir;
        let mut new_last_turn_bias = cd.last_turn_bias;

        match forward_or_backward {
            ForwardOrBackward::Forward => {
                step_forward_and_face_neighbor(
                    &mut new_pos,
                    &mut new_dir,
                    globe.spec().root_resolution,
                    &mut new_last_turn_bias,
                )
            }
            ForwardOrBackward::Backward => {
                step_backward_and_face_neighbor(
                    &mut new_pos,
                    &mut new_dir,
                    globe.spec().root_resolution,
                    &mut new_last_turn_bias,
                )
            }
        }.expect("CellDweller should have been in good state.");

        // Ask the globe if we can go there, attempting to climb up if there is a hil/cliff.
        // Usually we'll allow climbing a maximum of one block, but especially in certain tests
        // we want to let you climb higher!
        for _ in 0..(self.max_step_height + 1) {
            let cell = globe.maybe_non_authoritative_cell(new_pos);
            let can_move_to_cell = cell.material != Material::Dirt;

            if !can_move_to_cell {
                // Try again one higher.
                new_pos.z += 1;
                continue;
            }

            cd.set_cell_transform(new_pos, new_dir, new_last_turn_bias);
            // REVISIT: += ?
            cd.seconds_until_next_move = cd.seconds_between_moves;
            trace!(self.log, "Stepped"; "new_pos" => format!("{:?}", cd.pos()), "new_dir" => format!("{:?}", cd.dir()));

            break;
        }
    }
}

impl<'a> specs::System<'a> for MovementSystem {
    type SystemData = (
        Fetch<'a, TimeDeltaResource>,
        WriteStorage<'a, CellDweller>,
        WriteStorage<'a, Spatial>,
        ReadStorage<'a, Globe>,
        Fetch<'a, ActiveCellDweller>,
        FetchMut<'a, SendMessageQueue>,
    );

    fn run(&mut self, data: Self::SystemData) {
        self.consume_input();
        let (
            dt,
            mut cell_dwellers,
            mut spatials,
            globes,
            active_cell_dweller_resource,
            mut send_message_queue
        ) = data;
        let active_cell_dweller_entity = match active_cell_dweller_resource.maybe_entity {
            Some(entity) => entity,
            None => return,
        };
        let cd = cell_dwellers.get_mut(active_cell_dweller_entity).expect(
            "Someone deleted the controlled entity's CellDweller",
        );
        let spatial = spatials.get_mut(active_cell_dweller_entity).expect(
            "Someone deleted the controlled entity's Spatial",
        );

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
        let globe = match globes.get(globe_entity) {
            Some(globe) => globe,
            None => {
                warn!(
                    self.log,
                    "The globe associated with this CellDweller is not alive! Can't proceed!"
                );
                return;
            }
        };

        let mut maybe_did_something = false;

        // Count down until we're allowed to move next.
        if cd.seconds_until_next_move > 0.0 {
            cd.seconds_until_next_move = (cd.seconds_until_next_move - dt.0).max(0.0);
        }
        let still_waiting_to_move = cd.seconds_until_next_move > 0.0;
        // We can only step if forward XOR backward.
        // Otherwise we're not trying to go anywhere,
        // or we're trying to go both directions.
        let forward_xor_backward = self.step_forward != self.step_backward;
        if !still_waiting_to_move && forward_xor_backward {
            let forward_or_backward = if self.step_forward {
                ForwardOrBackward::Forward
            } else {
                ForwardOrBackward::Backward
            };
            maybe_did_something = true;
            self.step_if_possible(cd, globe, forward_or_backward);
        }

        // Count down until we're allowed to turn next.
        if cd.seconds_until_next_turn > 0.0 {
            cd.seconds_until_next_turn = (cd.seconds_until_next_turn - dt.0).max(0.0);
        }
        let still_waiting_to_turn = cd.seconds_until_next_turn > 0.0;
        if !still_waiting_to_turn {
            if self.turn_left && !self.turn_right {
                cd.turn(TurnDir::Left);
                cd.seconds_until_next_turn = cd.seconds_between_turns;
                trace!(self.log, "Turned left"; "new_pos" => format!("{:?}", cd.pos()), "new_dir" => format!("{:?}", cd.dir()));
                maybe_did_something = true;
            } else if self.turn_right && !self.turn_left {
                cd.turn(TurnDir::Right);
                cd.seconds_until_next_turn = cd.seconds_between_turns;
                trace!(self.log, "Turned right"; "new_pos" => format!("{:?}", cd.pos()), "new_dir" => format!("{:?}", cd.dir()));
                maybe_did_something = true;
            }
        }

        // Update real-space coordinates if necessary.
        // TODO: do this in a separate system; it needs to be done before
        // things are rendered, but there might be other effects like gravity,
        // enemies shunting the cell dweller around, etc. that happen
        // after control.
        if cd.is_real_space_transform_dirty() {
            // TEMPHACK: don't send if we definitely didn't move,
            // because even though we're supposed to be broadcasting,
            // I'm currently sending to peer 1, which might not exist yet.

            // TODO: better way of deciding whether
            // to send network message.
            // Tell all peers about our new position.
            if maybe_did_something && send_message_queue.has_consumer {
                send_message_queue.queue.push_back(
                    SendMessage {
                        // TODO: this should be broadcast;
                        // allow that in a SendMessage.
                        dest_peer_id: PeerId(1),
                        game_message: CellDwellerMessage::SetPos(SetPosMessage {
                            new_pos: cd.pos,
                            new_dir: cd.dir,
                            new_last_turn_bias: cd.last_turn_bias,
                        }),
                        transport: Transport::UDP,
                    }
                )
            }

            spatial.set_local_transform(cd.get_real_transform_and_mark_as_clean());
        }
    }
}
