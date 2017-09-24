use std::sync::mpsc;
use std::collections::vec_deque::VecDeque;

use specs;
use specs::{FetchMut};
use shred;
use slog::Logger;

use super::{GameMessage, RecvMessage, WireMessage, RecvWireMessage, RecvMessageQueue};

pub struct RecvSystem<G: GameMessage>{
    log: Logger,
    recv_wire_message_rx: mpsc::Receiver<RecvWireMessage<G>>,
    // Stored here so that clients can borrow and clone it for their own needs.
    recv_wire_message_tx: mpsc::Sender<RecvWireMessage<G>>,
}

// TODO: accept should_listen parameter and not listen otherwise.
impl<G> RecvSystem<G>
    where G: GameMessage
{
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> RecvSystem<G> {
        // Ensure RecvMessage ring buffer resource is registered.
        let res_id = shred::ResourceId::new::<RecvMessageQueue<G>>();
        if !world.res.has_value(res_id) {
            let recv_message_queue = RecvMessageQueue {
                queue: VecDeque::<RecvMessage<G>>::new()
            };
            // RAGH, VecDeque is not Sync. Why does it have to be sync? :(
            world.add_resource(recv_message_queue);
        }

        // Create channel for slurping network messages.
        let (tx, rx) = mpsc::channel();
        RecvSystem {
            log: parent_log.new(o!()),
            recv_wire_message_rx: rx,
            recv_wire_message_tx: tx,
        }
    }

    /// Borrow the sender end of the channel that provides incoming network
    /// messages to this system. You will almost always want to clone this
    /// and provide it to the network server.
    pub fn sender(&self) -> &mpsc::Sender<RecvWireMessage<G>> {
        &self.recv_wire_message_tx
    }
}

impl<'a, G> specs::System<'a> for RecvSystem<G>
    where G: GameMessage
{
    // TODO: require peer list as systemdata.
    type SystemData = (FetchMut<'a, RecvMessageQueue<G>>,);

    fn run(&mut self, data: Self::SystemData) {
        let (mut recv_message_queue,) = data;

        // Slurp everything the server sent us.
        loop {
            let recv_wire_message = match self.recv_wire_message_rx.try_recv() {
                Ok(recv_wire_message) => recv_wire_message,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => panic!("sender hung up"),
            };

            let src = recv_wire_message.src;
            let message = match recv_wire_message.message {
                Ok(message) => message,
                Err(_) => {
                    warn!(self.log, "Got garbled message"; "peer_addr" => format!("{:?}", src));
                    continue;
                }
            };

            let game_message = match message {
                WireMessage::Game(game_message) => game_message,
                _ => {
                    warn!(self.log, "Don't yet know how to do anything with non-game messages");
                    continue;
                }
            };

            // TODO: Verify authenticity of message sender.
            // (All messages sent over the wire should include this,
            // initially as a plain assertion of their identity, and eventually
            // at least HMAC.)

            // Re-wrap the message for consumption by other systems.
            // TODO: tack on the peer ID.
            let recv_message = RecvMessage {
                game_message: game_message,
            };
            recv_message_queue.queue.push_back(recv_message);

        }
    }
}
