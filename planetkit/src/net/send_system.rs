use std;
use std::collections::vec_deque::VecDeque;
use std::net::SocketAddr;

use specs;
use specs::{FetchMut};
use shred;
use slog::Logger;
use futures;

use super::{
    GameMessage,
    SendMessage,
    WireMessage,
    SendWireMessage,
    SendMessageQueue,
    NewPeer,
};

pub struct SendSystem<G: GameMessage>{
    log: Logger,
    send_udp_wire_message_tx: futures::sync::mpsc::Sender<SendWireMessage<G>>,
    // Only exists until taken by a client.
    send_udp_wire_message_rx: Option<futures::sync::mpsc::Receiver<SendWireMessage<G>>>,
    // Only exists until taken by a client.
    new_peer_tx: Option<std::sync::mpsc::Sender<NewPeer<G>>>,
    new_peer_rx: std::sync::mpsc::Receiver<NewPeer<G>>,
    // TODO: track actual peer addresses
    one_true_peer_addr: SocketAddr,
}

impl<G> SendSystem<G>
    where G: GameMessage
{
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> SendSystem<G> {
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
        let (tx, rx) = futures::sync::mpsc::channel::<SendWireMessage<G>>(1000);

        // Create channel for establishing new peer connections.
        let (new_peer_tx, new_peer_rx) =
            std::sync::mpsc::channel::<NewPeer<G>>();

        let system = SendSystem {
            log: parent_log.new(o!()),
            send_udp_wire_message_tx: tx,
            send_udp_wire_message_rx: Some(rx),
            new_peer_tx: Some(new_peer_tx),
            new_peer_rx: new_peer_rx,
            one_true_peer_addr: placeholder_addr,
        };
        system
    }

    pub fn take_send_udp_wire_message_rx(&mut self)
        -> Option<futures::sync::mpsc::Receiver<SendWireMessage<G>>>
    {
        self.send_udp_wire_message_rx.take()
    }

    pub fn take_new_peer_sender(&mut self)
        -> Option<std::sync::mpsc::Sender<NewPeer<G>>>
    {
        self.new_peer_tx.take()
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

        // TODO: while the new peers channel has anything in it...
        // - allocate next peer id
        // - store new channel for them.

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

            self.send_udp_wire_message_tx.try_send(send_wire_message).unwrap_or_else(|err| {
                error!(self.log, "Could send message to network server; was the buffer full?"; "err" => format!("{:?}", err));
                ()
            });
        }
    }
}
