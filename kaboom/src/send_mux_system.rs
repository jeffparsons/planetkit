use slog::Logger;
use specs;
use specs::Write;

use crate::pk::cell_dweller;
use crate::pk::net::{SendMessage, SendMessageQueue};

use crate::message::Message;

pub struct SendMuxSystem {
    log: Logger,
    initialized: bool,
}

impl SendMuxSystem {
    pub fn new(parent_log: &Logger) -> SendMuxSystem {
        SendMuxSystem {
            log: parent_log.new(o!()),
            initialized: false,
        }
    }
}

impl<'a> specs::System<'a> for SendMuxSystem {
    type SystemData = (
        Write<'a, SendMessageQueue<Message>>,
        Write<'a, cell_dweller::SendMessageQueue>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut send_message_queue, mut cell_dweller_send_queue) = data;

        if !self.initialized {
            // Signal to CellDweller module that we want it
            // to publish network messages.
            // TODO: Use shrev instead of this stuff.
            cell_dweller_send_queue.has_consumer = true;

            self.initialized = true;
        }

        // Drain the cell_dweller queue into the send_message queue.
        while let Some(message) = cell_dweller_send_queue.queue.pop_front() {
            trace!(self.log, "Forwarding cell dweller message to send message queue"; "message" => format!("{:?}", message));
            send_message_queue.queue.push_back(SendMessage {
                destination: message.destination,
                game_message: Message::CellDweller(message.game_message),
                transport: message.transport,
            });
        }
    }
}
