use specs;
use specs::{Fetch, FetchMut};
use slog::Logger;
use futures;

use super::{
    GameMessage,
    WireMessage,
    SendWireMessage,
    SendMessageQueue,
    RecvMessage,
    RecvMessageQueue,
    NetworkPeers,
    NetworkPeer,
    Destination,
    PeerId,
    Transport,
    NodeResource,
};

pub struct SendSystem<G: GameMessage>{
    log: Logger,
    send_udp_tx: futures::sync::mpsc::Sender<SendWireMessage<G>>,
}

impl<G> SendSystem<G>
    where G: GameMessage
{
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> SendSystem<G> {
        use auto_resource::AutoResource;

        // Ensure resources we use are present.
        SendMessageQueue::<G>::ensure(world);
        RecvMessageQueue::<G>::ensure(world);
        NetworkPeers::<G>::ensure(world);
        NodeResource::ensure(world);

        // Ensure ServerResource is present, and fetch the
        // channel ends we need from it.
        use super::ServerResource;
        let server_resource = ServerResource::<G>::ensure(world);
        let send_udp_tx = server_resource.send_udp_tx.clone();

        let system = SendSystem {
            log: parent_log.new(o!()),
            send_udp_tx: send_udp_tx,
        };
        system
    }

    fn send_message(&mut self, game_message: G, dest_peer: &mut NetworkPeer<G>, transport: Transport) {
        // Decide whether the message should go over TCP or UDP.
        match transport {
            Transport::UDP => {
                // Look up the destination socket address for this peer.
                // (Peer ID 0 refers to self, and isn't in the vec.)
                let dest_socket_addr = dest_peer.socket_addr;

                // Re-wrap the message for sending.
                let send_wire_message = SendWireMessage {
                    dest: dest_socket_addr,
                    message: WireMessage::Game(game_message),
                };

                self.send_udp_tx.try_send(send_wire_message).unwrap_or_else(|err| {
                    error!(self.log, "Could send message to UDP client; was the buffer full?"; "err" => format!("{:?}", err));
                    ()
                });
            },
            Transport::TCP => {
                // Look up TCP sender channel for this peer.
                // (Peer ID 0 refers to self, and isn't in the vec.)
                let sender = &mut dest_peer.tcp_sender;

                let wire_message = WireMessage::Game(game_message);
                sender.try_send(wire_message).unwrap_or_else(|err| {
                    error!(self.log, "Could send message to TCP client; was the buffer full?"; "err" => format!("{:?}", err));
                    ()
                });
            }
        }
    }
}

impl<'a, G> specs::System<'a> for SendSystem<G>
    where G: GameMessage
{
    type SystemData = (
        FetchMut<'a, SendMessageQueue<G>>,
        FetchMut<'a, RecvMessageQueue<G>>,
        FetchMut<'a, NetworkPeers<G>>,
        Fetch<'a, NodeResource>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut send_message_queue,
            mut recv_message_queue,
            mut network_peers,
            node_resource,
        ) = data;

        // Send everything in send queue to UDP/TCP server.
        while let Some(message) = send_message_queue.queue.pop_front() {
            // Re-wrap message and send it to its destination(s).
            match message.destination {
                Destination::One(peer_id) => {
                    // If the destination is ourself,
                    // then just put it straight back on the recv
                    // message queue. This is useful because being
                    // able to treat yourself as just another client/peer
                    // sometimes allows for more general code, rather than
                    // specialising between client or server case.
                    if peer_id.0 == 0 {
                        recv_message_queue.queue.push_back(
                            RecvMessage {
                                source: peer_id,
                                game_message: message.game_message,
                            }
                        );
                    } else {
                        self.send_message(
                            message.game_message,
                            &mut network_peers.peers[peer_id.0 as usize - 1],
                            message.transport,
                        );
                    }
                },
                Destination::EveryoneElse => {
                    for peer in network_peers.peers.iter_mut() {
                        self.send_message(
                            message.game_message.clone(),
                            peer,
                            message.transport,
                        );
                    }
                },
                Destination::EveryoneElseExcept(peer_id) => {
                    // Everyone except yourself and another specified peer.
                    // Useful if we just got an update (vs. polite request) from
                    // a client that we don't intend to challenge (e.g. "I moved here" ... "ok"),
                    // and just want to forward on to all other clients.
                    for peer in network_peers.peers.iter_mut() {
                        if peer.id == peer_id || peer.id.0 == 0 {
                            continue;
                        }
                        self.send_message(
                            message.game_message.clone(),
                            peer,
                            message.transport,
                        );
                    }
                },
                Destination::Master => {
                    if node_resource.is_master {
                        recv_message_queue.queue.push_back(
                            RecvMessage {
                                source: PeerId(0),
                                game_message: message.game_message,
                            }
                        );
                    } else {
                        for peer in network_peers.peers.iter_mut() {
                            self.send_message(
                                message.game_message.clone(),
                                peer,
                                message.transport,
                            );
                        }
                    }
                },
                Destination::EveryoneIncludingSelf => {
                    // Send to everyone else.
                    for peer in network_peers.peers.iter_mut() {
                        self.send_message(
                            message.game_message.clone(),
                            peer,
                            message.transport,
                        );
                    }
                    // Send to self.
                    recv_message_queue.queue.push_back(
                        RecvMessage {
                            source: PeerId(0),
                            game_message: message.game_message,
                        }
                    );
                }
            }
        }
    }
}
