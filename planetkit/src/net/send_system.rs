use std::collections::vec_deque::VecDeque;
use std::net::SocketAddr;

use specs;
use specs::{FetchMut};
use shred;
use slog::Logger;
use futures::sync;

use super::{GameMessage, SendMessage, WireMessage, SendWireMessage, SendMessageQueue};

pub struct SendSystem<G: GameMessage>{
    log: Logger,
    // TODO: separate channels for sending over TCP and UDP.
    send_wire_message_tx: sync::mpsc::Sender<SendWireMessage<G>>,
    // TODO: track actual peer addresses
    one_true_peer_addr: SocketAddr,
}

impl<G> SendSystem<G>
    where G: GameMessage
{
    pub fn new(parent_log: &Logger, world: &mut specs::World)
            -> (SendSystem<G>, sync::mpsc::Receiver<SendWireMessage<G>>) {
        // Ensure SendMessage ring buffer resource is registered.
        let res_id = shred::ResourceId::new::<SendMessageQueue<G>>();
        if !world.res.has_value(res_id) {
            let send_message_queue = SendMessageQueue {
                queue: VecDeque::<SendMessage<G>>::new()
            };
            world.add_resource(send_message_queue);
        }

        // TEMP
        let placeholder_addr = "0.0.0.0:0".to_string();
        let placeholder_addr = placeholder_addr.parse::<SocketAddr>().unwrap();

        // Create channel for sending network messages.
        // TODO: how big is reasonable? Just go unbounded?
        let (tx, rx) = sync::mpsc::channel::<SendWireMessage<G>>(1000);
        let system = SendSystem {
            log: parent_log.new(o!()),
            send_wire_message_tx: tx,
            one_true_peer_addr: placeholder_addr,
        };
        (system, rx)
    }

    pub fn set_one_true_peer_addr(&mut self, addr: SocketAddr) {
        self.one_true_peer_addr = addr;
    }
}

impl<'a, G> specs::System<'a> for SendSystem<G>
    where G: GameMessage
{
    // TODO: require peer list as systemdata.
    type SystemData = (FetchMut<'a, SendMessageQueue<G>>,);

    fn run(&mut self, data: Self::SystemData) {
        let (mut send_message_queue,) = data;

        // Send everything in send queue to UDP server.
        while let Some(message) = send_message_queue.queue.pop_front() {
            ///
            // TODO: tack on the appropriate peer address
            // so we know where to send it!!!!!!!!!!!!!!!
            // For now, just use the hard-coded one provided.
            //

            // Re-wrap the message for sending.
            let send_wire_message = SendWireMessage {
                dest: self.one_true_peer_addr,
                message: WireMessage::Game(message.game_message),
            };

            // TODO: decide whether it should go over TCP or UDP

            self.send_wire_message_tx.try_send(send_wire_message).unwrap_or_else(|err| {
                error!(self.log, "Could send message to network server; was the buffer full?"; "err" => format!("{:?}", err));
                ()
            });
        }
    }
}
