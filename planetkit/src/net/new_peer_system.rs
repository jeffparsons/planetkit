use std;
use std::sync::mpsc::TryRecvError;

use specs;
use specs::WriteExpect;
use slog::Logger;

use super::{
    GameMessage,
    NewPeer,
    NetworkPeers,
    NetworkPeer,
    PeerId,
};

pub struct NewPeerSystem<G: GameMessage>{
    _log: Logger,
    new_peer_rx: std::sync::mpsc::Receiver<NewPeer<G>>,
}

impl<G> NewPeerSystem<G>
    where G: GameMessage
{
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> NewPeerSystem<G> {
        use auto_resource::AutoResource;

        // Ensure ServerResource is present, and fetch the
        // channel ends we need from it.
        use super::ServerResource;
        let server_resource = ServerResource::<G>::ensure(world);
        let new_peer_rx = server_resource.new_peer_rx
            .lock()
            .expect("Couldn't get lock on new peer receiver")
            .take()
            .expect("Somebody already took it!");

        let system = NewPeerSystem {
            _log: parent_log.new(o!()),
            new_peer_rx: new_peer_rx,
        };
        system
    }
}

impl<'a, G> specs::System<'a> for NewPeerSystem<G>
    where G: GameMessage
{
    type SystemData = (
        WriteExpect<'a, NetworkPeers<G>>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut network_peers,
        ) = data;

        // Register any new peers that have connected
        // (or that we've connected to).
        loop {
            match self.new_peer_rx.try_recv() {
                Ok(new_peer) => {
                    // Peer ID 0 refers to self, and isn't in the array.
                    let next_peer_id = PeerId(network_peers.peers.len() as u16 + 1);
                    let peer = NetworkPeer {
                        id: next_peer_id,
                        tcp_sender: new_peer.tcp_sender,
                        socket_addr: new_peer.socket_addr,
                    };
                    network_peers.peers.push(peer);

                    // Cool, we've registered the peer, so we can now
                    // handle messages from the network. Let the network
                    // bits know that.
                    new_peer.ready_to_receive_tx.send(()).expect("Receiver hung up?");

                    // Leave a note about the new peer so game-specific
                    // systems can do whatever initialization they might
                    // need to do.
                    //
                    // TODO: don't do this until we've heard from the peer
                    // that they are ready to receive messages. Otherwise
                    // we might start sending them things over UDP that
                    // they're not ready to receive, and they'll spew a bunch
                    // of unnecessary warnings. :)
                    network_peers.new_peers.push_back(next_peer_id);
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
    }
}
