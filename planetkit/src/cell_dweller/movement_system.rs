use std::sync::mpsc;
use specs;
use specs::{Read, ReadStorage, WriteStorage, Write};
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
    Destination,
    NetMarker,
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
        use piston::input::{Button, ButtonState};
        use piston::input::keyboard::Key;

        if let &Input::Button(button_args) = input_event {
            if let Button::Keyboard(key) = button_args.button {
                let is_down = match button_args.state {
                    ButtonState::Press => true,
                    ButtonState::Release => false,
                };
                match key {
                    // Arrow keys.
                    Key::Up => self.sender.send(MovementEvent::StepForward(is_down)).unwrap(),
                    Key::Down => self.sender.send(MovementEvent::StepBackward(is_down)).unwrap(),
                    Key::Left => self.sender.send(MovementEvent::TurnLeft(is_down)).unwrap(),
                    Key::Right => self.sender.send(MovementEvent::TurnRight(is_down)).unwrap(),
                    // IJKL keys.
                    Key::I => self.sender.send(MovementEvent::StepForward(is_down)).unwrap(),
                    Key::K => self.sender.send(MovementEvent::StepBackward(is_down)).unwrap(),
                    Key::J => self.sender.send(MovementEvent::TurnLeft(is_down)).unwrap(),
                    Key::L => self.sender.send(MovementEvent::TurnRight(is_down)).unwrap(),
                    // WASD keys.
                    Key::W => self.sender.send(MovementEvent::StepForward(is_down)).unwrap(),
                    Key::S => self.sender.send(MovementEvent::StepBackward(is_down)).unwrap(),
                    Key::A => self.sender.send(MovementEvent::TurnLeft(is_down)).unwrap(),
                    Key::D => self.sender.send(MovementEvent::TurnRight(is_down)).unwrap(),
                    _ => (),
                }
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
        input_receiver: mpsc::Receiver<MovementEvent>,
        parent_log: &Logger,
    ) -> MovementSystem {
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
        let under_cell = match globe.maybe_non_authoritative_cell(under_pos) {
            Ok(cell) => cell,
            // Chunk not loaded; wait until it is before attempting to move.
            Err(_) => return,
        };
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
            let cell = match globe.maybe_non_authoritative_cell(new_pos) {
                Ok(cell) => cell,
                // Chunk not loaded; wait until it is before attempting to move.
                Err(_) => return,
            };
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
        Read<'a, TimeDeltaResource>,
        WriteStorage<'a, CellDweller>,
        WriteStorage<'a, Spatial>,
        ReadStorage<'a, Globe>,
        Read<'a, ActiveCellDweller>,
        Write<'a, SendMessageQueue>,
        ReadStorage<'a, NetMarker>,
    );

    fn run(&mut self, data: Self::SystemData) {
        self.consume_input();
        let (
            dt,
            mut cell_dwellers,
            mut spatials,
            globes,
            active_cell_dweller_resource,
            mut send_message_queue,
            net_markers,
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
            } else if self.turn_right && !self.turn_left {
                cd.turn(TurnDir::Right);
                cd.seconds_until_next_turn = cd.seconds_between_turns;
                trace!(self.log, "Turned right"; "new_pos" => format!("{:?}", cd.pos()), "new_dir" => format!("{:?}", cd.dir()));
            }
        }

        // Update real-space coordinates if necessary.
        // TODO: do this in a separate system; it needs to be done before
        // things are rendered, but there might be other effects like gravity,
        // enemies shunting the cell dweller around, etc. that happen
        // after control.
        if cd.is_real_space_transform_dirty() {
            // TODO: better way of deciding whether
            // to send network message. Using `is_real_space_transform_dirty` is a haaaack.
            // Tell all peers about our new position.
            if send_message_queue.has_consumer {
                // If there's a network consumer, then presumably
                // the entity has been given a global ID.
                let entity_id = net_markers
                    .get(active_cell_dweller_entity)
                    .expect("Shouldn't be trying to tell peers about entities that don't have global IDs!")
                    .id;
                // TODO: this shouldn't even be a network message;
                // it should be an EVENT on a pubsub thing (or similar...
                // you don't want to accidentally miss it this frame
                // if you're using asynchronous channels -- is this
                // actually a serious consideration?). Then if there's
                // a network system hooked up, then it can broadcast it.
                send_message_queue.queue.push_back(
                    SendMessage {
                        // In practice, if you're the server this will mean "all clients"
                        // because all of them need to know about the change, and if you're
                        // a client then for now your only peer will be the server.
                        // All of this will obviously need to be revisited if we allow
                        // connecting to multiple servers, or to other non-server peers.
                        destination: Destination::EveryoneElse,
                        game_message: CellDwellerMessage::SetPos(SetPosMessage {
                            entity_id: entity_id,
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
