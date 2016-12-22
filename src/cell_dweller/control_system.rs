use std::sync::mpsc;
use specs;
use slog::Logger;

use types::*;
use super::cell_dweller::CellDweller;

pub enum Event {
    MoveForward(bool),
}

pub struct ControlSystem {
    input_receiver: mpsc::Receiver<Event>,
    log: Logger,
    move_forward: bool,
}

impl ControlSystem {
    pub fn new(input_receiver: mpsc::Receiver<Event>, parent_log: &Logger) -> ControlSystem {
        ControlSystem {
            input_receiver: input_receiver,
            log: parent_log.new(o!()),
            move_forward: false,
        }
    }

    fn consume_input(&mut self) {
        loop {
            match self.input_receiver.try_recv() {
                Ok(Event::MoveForward(b)) => self.move_forward = b,
                Err(_) => return,
            }
        }
    }
}

impl specs::System<TimeDelta> for ControlSystem {
    fn run(&mut self, arg: specs::RunArg, _dt: TimeDelta) {
        use specs::Join;
        self.consume_input();
        let mut cell_dwellers = arg.fetch(|w|
            w.write::<CellDweller>()
        );
        for cd in (&mut cell_dwellers).iter() {
            if self.move_forward {
                // TODO: only step forward if it's been long enough since last step.
                cd.temp_advance_pos();
                debug!(self.log, "Stepped"; "new_pos" => format!("{:?}", cd.pos()));
            }
        }
    }
}
