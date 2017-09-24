mod recv_system;
mod send_system;
mod udp_server;
mod tcp_server;

#[cfg(test)]
mod tests;

use std::fmt::Debug;
use std::net::SocketAddr;
use std::collections::vec_deque::VecDeque;

use serde::Serialize;
use serde::de::DeserializeOwned;

pub use self::recv_system::RecvSystem;
pub use self::send_system::SendSystem;
pub use self::udp_server::start_udp_server;
pub use self::tcp_server::start_tcp_server;

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
pub trait GameMessage : 'static + Serialize + DeserializeOwned + Debug + Eq + PartialEq + Send + Sync {}

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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct SendWireMessage<G> {
    dest: SocketAddr,
    message: WireMessage<G>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RecvMessage<G> {
    // TODO: sender peer id
    game_message: G,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct SendMessage<G> {
    // TODO: dest peer id
    game_message: G,
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
