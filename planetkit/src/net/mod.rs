// NOTE: Lots of this stuff doesn't work on the web yet.
// Most of the module is disabled for Emscripten.

#[cfg(not(target_os="emscripten"))] mod recv_system;
#[cfg(not(target_os="emscripten"))] mod send_system;
#[cfg(not(target_os="emscripten"))] mod server;
#[cfg(not(target_os="emscripten"))] mod server_resource;
#[cfg(not(target_os="emscripten"))] mod udp;
#[cfg(not(target_os="emscripten"))] mod tcp;

#[cfg(test)]
mod tests;

use std::fmt::Debug;
use std::net::SocketAddr;
use std::collections::vec_deque::VecDeque;
use std::collections::HashMap;
use std::ops::Range;

use serde::Serialize;
use serde::de::DeserializeOwned;
use futures;
use specs;

use ::AutoResource;
#[cfg(not(target_os="emscripten"))] pub use self::recv_system::RecvSystem;
#[cfg(not(target_os="emscripten"))] pub use self::send_system::SendSystem;
#[cfg(not(target_os="emscripten"))] pub use self::server::Server;
#[cfg(not(target_os="emscripten"))] pub use self::server_resource::ServerResource;

// TODO: all this naming is pretty shoddy, and evolved in an awkward
// way that makes it super unclear what's for what.

// Game-specific message body.
//
// These are forwarded to systems without any filtering or sanitization
// by generic network systems. Therefore they should in general be treated
// as a verbatim message from a peer that is only trusted as much as that
// peer is trusted.
//
// Exists primarily as a way to aggregate all the super-traits we expect,
// especially around being able to serialize it.
pub trait GameMessage : 'static + Serialize + DeserializeOwned + Debug + Eq + PartialEq + Send + Sync + Clone {}

// TODO: identify self in every message. Make this a struct wrapping the enum,
// or include your identity in Goodbye and a Game wrapper?
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum WireMessage<G> {
    /// First message you should send to any peer when establishing a connection
    /// (keeping in mind that this is only a logical connection in PlanetKit, not a stateful TCP connection)
    /// regardless of the roles each peer might have (server, client, equal).
    Hello,
    /// Courtesy message before disconnecting, so that your peer can regard
    /// you as having cleanly disconnected rather than mysteriously disappearing.
    Goodbye,
    /// Game-specific message, opaque to PlanetKit aside from the constraints
    /// placed on it by `GameMessage`.
    Game(G),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RecvWireMessage<G> {
    src: SocketAddr,
    // TODO: error type for mangled message
    message: Result<WireMessage<G>, ()>,
}

// Only actually used for UDP; for TCP messages there are
// per-peer channels all the way to the SendSystem, so there's
// no need for an extra envelope around the message.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct SendWireMessage<G> {
    dest: SocketAddr,
    message: WireMessage<G>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RecvMessage<G> {
    pub source: PeerId,
    pub game_message: G,
}

#[derive(Debug, Clone, Copy)]
pub enum Transport {
    UDP,
    TCP,
}

/// Game message wrapped for sending to peer(s).
/// Might wrap a module's message, or a game's message.
#[derive(Debug)]
pub struct SendMessage<G> {
    pub destination: Destination,
    pub game_message: G,
    /// The network transport that should be used to send this message.
    pub transport: Transport,
}

#[derive(Debug)]
pub enum Destination {
    One(PeerId),
    EveryoneElse,
    // Useful if you're the server and a client just told you
    // something that everyone else needs to know.
    EveryoneElseExcept(PeerId),
    // Send to the master, whether that is someone else or this node itself.
    Master,
    // TODO: consider adding a new one for "all including self"
    // so we can simplify some code paths that work differently
    // depending on whether you're the server or not.
}

/// `World`-global resource for game messages waiting to be dispatched
/// to game-specific systems.
pub struct RecvMessageQueue<G> {
    pub queue: VecDeque<RecvMessage<G>>,
}

/// `World`-global resource for game messages waiting to be sent
/// to peers.
pub struct SendMessageQueue<G> {
    pub queue: VecDeque<SendMessage<G>>,
}

impl<G: GameMessage> AutoResource for SendMessageQueue<G> {
    fn new(_world: &mut specs::World) -> SendMessageQueue<G> {
        SendMessageQueue {
            queue: VecDeque::<SendMessage<G>>::new(),
        }
    }
}

/// Local identifier for a network peer.
///
/// This identifier is used to label network peers
/// within this host; i.e. it should never be communicated
/// to a peer.
///
/// Note that this is not the same thing as a player ID.
/// This is used in deciding what network peer to send
/// messages to, and which peers have authority over what
/// parts of the world. We might receive messages regarding
/// multiple players from one peer, and need to decide
/// whether that peer has authority to make assertions about
/// those players.
#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct PeerId(pub u16);

/// A new network peer.
///
/// Might be a server we connected to,
/// or a client that connected to us.
/// Contains the peer's address, and a channel used
/// to send it message over TCP.
///
/// This is used to communicate these essentials
/// to the `SendSystem` when a new connection is established.
pub struct NewPeer<G> {
    pub tcp_sender: futures::sync::mpsc::Sender<WireMessage<G>>,
    pub socket_addr: SocketAddr,
    // Fires when the RecvSystem is ready to receive messages from
    // the network. This gives it a chance to register the peer so
    // that it can identify messages from it. Asynchronous programming
    // is fuuuuun.
    pub ready_to_receive_tx: futures::sync::oneshot::Sender<()>,
}

pub struct NetworkPeer<G> {
    pub id: PeerId,
    pub tcp_sender: futures::sync::mpsc::Sender<WireMessage<G>>,
    pub socket_addr: SocketAddr,
    // TODO: connection state, etc.
}

/// `World`-global resource for network peers.
pub struct NetworkPeers<G> {
    pub peers: Vec<NetworkPeer<G>>,
    // List of new peers for a single game-specific
    // system to use.
    // TODO: This makes yet another good use case for some kind
    // of pub/sub event system.
    pub new_peers: VecDeque<PeerId>,
}

impl<G: GameMessage> AutoResource for NetworkPeers<G> {
    fn new(_world: &mut specs::World) -> NetworkPeers<G> {
        NetworkPeers {
            peers: Vec::<NetworkPeer<G>>::new(),
            new_peers: VecDeque::<PeerId>::new(),
        }
    }
}

/// `World`-global resource for global entity naming.
pub struct EntityIds {
    // Range of IDs this node can allocate for itself.
    // The master will tell you what this should be.
    pub range: Range<u64>,
    // Only entities to be sent over the network should
    // be given global identities in this mapping.
    pub mapping: HashMap<u64, specs::Entity>,
}

impl AutoResource for EntityIds {
    fn new(_world: &mut specs::World) -> EntityIds {
        EntityIds {
            // The master will tell us what our namespace is.
            // TODO: make the master actually do that.
            range: 0..100,
            mapping: HashMap::new(),
        }
    }
}

pub struct NetMarker {
    pub id: u64,
    // TODO: sequence number so we can reject old updates?
}

impl specs::Component for NetMarker {
    type Storage = specs::DenseVecStorage<Self>;
}

/// Local state for this network node.
/// Used by some systems even if we're only running the game locally,
/// because there are some generic systems (e.g. CellDweller mining)
/// that need to know whether we are the master.
pub struct NodeResource {
    // Are we the server/owner of the game?
    pub is_master: bool,
}

impl AutoResource for NodeResource {
    fn new(_world: &mut specs::World) -> NodeResource {
        NodeResource {
            // This will get set to something meaningful
            // when hosting/joining a game.
            is_master: false,
        }
    }
}
