use specs;
use specs::{Write, WriteExpect};
use slog::Logger;

use pk::cell_dweller;
use pk::net::{
    SendMessage,
    SendMessageQueue,
};

use ::message::Message;

pub struct SendMuxSystem{
    log: Logger,
}

impl SendMuxSystem {
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> SendMuxSystem {
        use pk::AutoResource;

        // Signal to CellDweller module that we want it
        // to publish network messages.
        let mut cell_dweller_queue =
            cell_dweller::SendMessageQueue::ensure(world);
        cell_dweller_queue.has_consumer = true;

        SendMuxSystem {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for SendMuxSystem {
    type SystemData = (
        Write<'a, SendMessageQueue<Message>>,
        WriteExpect<'a, cell_dweller::SendMessageQueue>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut send_message_queue,
            mut cell_dweller_send_queue,
        ) = data;

        // Drain the cell_dweller queue into the send_message queue.
        while let Some(message) = cell_dweller_send_queue.queue.pop_front() {
            trace!(self.log, "Forwarding cell dweller message to send message queue"; "message" => format!("{:?}", message));
            send_message_queue.queue.push_back(
                SendMessage {
                    destination: message.destination,
                    game_message: Message::CellDweller(message.game_message),
                    transport: message.transport,
                }
            );
        }
    }
}
