use std::sync::mpsc;
use specs;
use slog::Logger;

use types::*;
use super::CellDweller;
use ::Spatial;

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

            if self.move_forward && !still_waiting_to_move {
                cd.temp_advance_pos();
                debug!(self.log, "Stepped"; "new_pos" => format!("{:?}", cd.pos()));
                cd.seconds_until_next_move = cd.seconds_between_moves;
            }

            // Update real-space coordinates if necessary.
            // TODO: do this in a separate system; it needs to be done before
            // things are rendered, but there might be other effects like gravity,
            // enemies shunting the cell dweller around, etc. that happen
            // after control.
            if cd.is_real_space_transform_dirty() {
                spatial.pos = cd.get_real_transform_and_mark_as_clean();
            }
        }
    }
}
