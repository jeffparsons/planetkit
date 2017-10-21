use std::sync::mpsc;
use std::collections::vec_deque::VecDeque;

use specs;
use specs::{FetchMut};
use shred;
use slog::Logger;

use super::{GameMessage, RecvMessage, WireMessage, RecvWireMessage, RecvMessageQueue};

pub struct RecvSystem<G: GameMessage>{
    log: Logger,
    // Channel for slurping wire messages from network server.
    recv_rx: mpsc::Receiver<RecvWireMessage<G>>,
}

impl<G> RecvSystem<G>
    where G: GameMessage
{
    pub fn new(
        parent_log: &Logger,
        world: &mut specs::World,
    ) -> RecvSystem<G> {
        use auto_resource::AutoResource;

        // Ensure RecvMessage ring buffer resource is registered.
        // TODO: make this a self-ensuring resource.
        let res_id = shred::ResourceId::new::<RecvMessageQueue<G>>();
        if !world.res.has_value(res_id) {
            let recv_message_queue = RecvMessageQueue {
                queue: VecDeque::<RecvMessage<G>>::new()
            };
            world.add_resource(recv_message_queue);
        }

        // Ensure ServerResource is present, and fetch the
        // wire message receiver from it.
        use super::ServerResource;
        let server_resource = ServerResource::<G>::ensure(world);
        let recv_rx = server_resource.recv_rx
            .lock()
            .expect("Couldn't get lock on wire message receiver")
            .take()
            .expect("Somebody already took it!");

        RecvSystem {
            log: parent_log.new(o!()),
            recv_rx: recv_rx,
        }
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
            let recv_wire_message = match self.recv_rx.try_recv() {
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
