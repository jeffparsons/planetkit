use std;
use std::collections::vec_deque::VecDeque;
use std::sync::mpsc::TryRecvError;

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
    NetworkPeers,
    NetworkPeer,
    PeerId,
};

pub struct SendSystem<G: GameMessage>{
    log: Logger,
    send_udp_wire_message_tx: futures::sync::mpsc::Sender<SendWireMessage<G>>,
    // Only exists until taken by a client.
    send_udp_wire_message_rx: Option<futures::sync::mpsc::Receiver<SendWireMessage<G>>>,
    // Only exists until taken by a client.
    new_peer_tx: Option<std::sync::mpsc::Sender<NewPeer<G>>>,
    new_peer_rx: std::sync::mpsc::Receiver<NewPeer<G>>,
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

        // TODO: make a generic helper for this!!!
        // Ensure NetworkPeers resource is registered.
        let res_id = shred::ResourceId::new::<NetworkPeers<G>>();
        if !world.res.has_value(res_id) {
            let network_peers = NetworkPeers {
                peers: Vec::<NetworkPeer<G>>::new()
            };
            world.add_resource(network_peers);
        }

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
}

impl<'a, G> specs::System<'a> for SendSystem<G>
    where G: GameMessage
{
    // TODO: require peer list as systemdata.
    type SystemData = (
        FetchMut<'a, SendMessageQueue<G>>,
        FetchMut<'a, NetworkPeers<G>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut send_message_queue,
            mut peers,
        ) = data;
        let peers = &mut peers.peers;

        // TODO: does this stuff even belong here,
        // or should things like `send_system_new_peer_sender`
        // be in RESOURCES? I think the latter.
        // Then you can have a NewPeerSystem or something.
        // TODO: try that.

        // Register any new peers that have connected
        // (or that we've connected to).
        loop {
            match self.new_peer_rx.try_recv() {
                Ok(new_peer) => {
                    // Peer ID 0 refers to self, and isn't in the array.
                    let next_peer_id = PeerId(peers.len() as u16 + 1);
                    let peer = NetworkPeer {
                        id: next_peer_id,
                        tcp_sender: new_peer.tcp_sender,
                        socket_addr: new_peer.socket_addr,
                    };
                    peers.push(peer);
                },
                Err(err) => {
                    match err {
                        TryRecvError::Empty => {
                            break;
                        },
                        TryRecvError::Disconnected => {
                            // TODO: don't panic; we're going to need
                            // a way to shut the server down gracefully.
                            panic!("Sender hung up");
                        },
                    }
                },
            }
        }

        // Send everything in send queue to UDP/TCP server.
        while let Some(message) = send_message_queue.queue.pop_front() {
            // TODO: decide whether it should go over TCP or UDP

            // Look up the destination socket address for this peer.
            // Peer ID 0 refers to self, and isn't in the vec.
            let dest_socket_addr = peers[message.dest_peer_id.0 as usize - 1].socket_addr;

            // Re-wrap the message for sending.
            let send_wire_message = SendWireMessage {
                dest: dest_socket_addr,
                message: WireMessage::Game(message.game_message),
            };

            self.send_udp_wire_message_tx.try_send(send_wire_message).unwrap_or_else(|err| {
                error!(self.log, "Could send message to network server; was the buffer full?"; "err" => format!("{:?}", err));
                ()
            });
        }
    }
}
