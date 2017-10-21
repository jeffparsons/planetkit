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
    Transport,
};

pub struct SendSystem<G: GameMessage>{
    log: Logger,
    send_udp_tx: futures::sync::mpsc::Sender<SendWireMessage<G>>,
    new_peer_rx: std::sync::mpsc::Receiver<NewPeer<G>>,
}

impl<G> SendSystem<G>
    where G: GameMessage
{
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> SendSystem<G> {
        use auto_resource::AutoResource;

        // Ensure SendMessage ring buffer resource is registered.
        let res_id = shred::ResourceId::new::<SendMessageQueue<G>>();
        if !world.res.has_value(res_id) {
            let send_message_queue = SendMessageQueue {
                queue: VecDeque::<SendMessage<G>>::new()
            };
            world.add_resource(send_message_queue);
        }

        // TODO: make a generic helper for this!!!
        // TODO: just use autoresoruce.
        // Ensure NetworkPeers resource is registered.
        let res_id = shred::ResourceId::new::<NetworkPeers<G>>();
        if !world.res.has_value(res_id) {
            let network_peers = NetworkPeers {
                peers: Vec::<NetworkPeer<G>>::new()
            };
            world.add_resource(network_peers);
        }

        // Ensure ServerResource is present, and fetch the
        // channel ends we need from it.
        use super::ServerResource;
        let server_resource = ServerResource::<G>::ensure(world);
        let send_udp_tx = server_resource.send_udp_tx.clone();
        let new_peer_rx = server_resource.new_peer_rx
            .lock()
            .expect("Couldn't get lock on new peer receiver")
            .take()
            .expect("Somebody already took it!");

        let system = SendSystem {
            log: parent_log.new(o!()),
            send_udp_tx: send_udp_tx,
            new_peer_rx: new_peer_rx,
        };
        system
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
            // Decide whether the message should go over TCP or UDP.
            match message.transport {
                Transport::UDP => {
                    // Look up the destination socket address for this peer.
                    // (Peer ID 0 refers to self, and isn't in the vec.)
                    let dest_socket_addr = peers[message.dest_peer_id.0 as usize - 1].socket_addr;

                    // Re-wrap the message for sending.
                    let send_wire_message = SendWireMessage {
                        dest: dest_socket_addr,
                        message: WireMessage::Game(message.game_message),
                    };

                    self.send_udp_tx.try_send(send_wire_message).unwrap_or_else(|err| {
                        error!(self.log, "Could send message to UDP client; was the buffer full?"; "err" => format!("{:?}", err));
                        ()
                    });
                },
                Transport::TCP => {
                    // Look up TCP sender channel for this peer.
                    // (Peer ID 0 refers to self, and isn't in the vec.)
                    let sender = &mut peers[message.dest_peer_id.0 as usize - 1].tcp_sender;

                    let wire_message = WireMessage::Game(message.game_message);
                    sender.try_send(wire_message).unwrap_or_else(|err| {
                        error!(self.log, "Could send message to TCP client; was the buffer full?"; "err" => format!("{:?}", err));
                        ()
                    });
                }
            }
        }
    }
}
