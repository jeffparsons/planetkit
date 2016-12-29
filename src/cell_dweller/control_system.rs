use std::sync::mpsc;
use specs;
use slog::Logger;

use types::*;
use super::CellDweller;
use ::Spatial;
use ::movement::*;

pub enum Event {
    StepForward(bool),
    StepBackward(bool),
    TurnLeft(bool),
    TurnRight(bool),
}

pub struct ControlSystem {
    input_receiver: mpsc::Receiver<Event>,
    log: Logger,
    step_forward: bool,
    step_backward: bool,
    turn_left: bool,
    turn_right: bool,
}

impl ControlSystem {
    pub fn new(input_receiver: mpsc::Receiver<Event>, parent_log: &Logger) -> ControlSystem {
        ControlSystem {
            input_receiver: input_receiver,
            log: parent_log.new(o!()),
            step_forward: false,
            step_backward: false,
            turn_left: false,
            turn_right: false,
        }
    }

    fn consume_input(&mut self) {
        loop {
            match self.input_receiver.try_recv() {
                Ok(Event::StepForward(b)) => self.step_forward = b,
                Ok(Event::StepBackward(b)) => self.step_backward = b,
                Ok(Event::TurnLeft(b)) => self.turn_left = b,
                Ok(Event::TurnRight(b)) => self.turn_right = b,
                Err(_) => return,
            }
        }
    }
}

impl specs::System<TimeDelta> for ControlSystem {
    fn run(&mut self, arg: specs::RunArg, dt: TimeDelta) {
        use specs::Join;
        self.consume_input();
        let (mut cell_dwellers, mut spatials) = arg.fetch(|w|
            (w.write::<CellDweller>(), w.write::<Spatial>())
        );
        for (cd, spatial) in (&mut cell_dwellers, &mut spatials).iter() {
            // Count down until we're allowed to move next.
            if cd.seconds_until_next_move > 0.0 {
                cd.seconds_until_next_move = (cd.seconds_until_next_move - dt).max(0.0);
            }
            let still_waiting_to_move = cd.seconds_until_next_move > 0.0;
            if !still_waiting_to_move {
                if self.step_forward && !self.step_backward  {
                    cd.step_forward();
                    cd.seconds_until_next_move = cd.seconds_between_moves;
                    trace!(self.log, "Stepped forward"; "new_pos" => format!("{:?}", cd.pos()), "new_dir" => format!("{:?}", cd.dir()));
                } else if self.step_backward && !self.step_forward {
                    cd.step_backward();
                    cd.seconds_until_next_move = cd.seconds_between_moves;
                    trace!(self.log, "Stepped backward"; "new_pos" => format!("{:?}", cd.pos()), "new_dir" => format!("{:?}", cd.dir()));
                }
            }

            // Count down until we're allowed to turn next.
            if cd.seconds_until_next_turn > 0.0 {
                cd.seconds_until_next_turn = (cd.seconds_until_next_turn - dt).max(0.0);
            }
            let still_waiting_to_turn = cd.seconds_until_next_turn > 0.0;
            if !still_waiting_to_turn {
                if self.turn_left && !self.turn_right  {
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
                spatial.transform = cd.get_real_transform_and_mark_as_clean();
            }
        }
    }
}
