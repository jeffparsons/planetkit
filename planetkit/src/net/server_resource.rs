use futures::sync::mpsc as futmpsc;
use slog::Logger;
use std::sync::mpsc as stdmpsc;
use std::sync::{Arc, Mutex};

use super::{GameMessage, NewPeer, RecvWireMessage, SendWireMessage, Server};

pub struct ServerResource<G> {
    pub server: Arc<Mutex<Server<G>>>,
    // Only exists until taken by `System` that needs it.
    pub recv_rx: Arc<Mutex<Option<stdmpsc::Receiver<RecvWireMessage<G>>>>>,
    // Only exists until taken by `System` that needs it.
    pub new_peer_rx: Arc<Mutex<Option<stdmpsc::Receiver<NewPeer<G>>>>>,
    // Can be cloned; keep a copy forever.
    pub send_udp_tx: futmpsc::Sender<SendWireMessage<G>>,
}

impl<G: GameMessage> ServerResource<G> {
    // Can't implement Default because it needs a
    // root logger provided from the outside world.
    pub fn new(parent_log: &Logger) -> ServerResource<G> {
        // Create all the various channels we'll
        // need to link up the `Server`, `RecvSystem`,
        // and `SendSystem`.
        let (recv_tx, recv_rx) = stdmpsc::channel::<RecvWireMessage<G>>();
        let (new_peer_tx, new_peer_rx) = stdmpsc::channel::<NewPeer<G>>();
        // TODO: how big is reasonable? Just go unbounded?
        let (send_udp_tx, send_udp_rx) = futmpsc::channel::<SendWireMessage<G>>(1000);

        let server = Server::<G>::new(&parent_log, recv_tx, new_peer_tx, send_udp_rx);
        ServerResource {
            server: Arc::new(Mutex::new(server)),
            recv_rx: Arc::new(Mutex::new(Some(recv_rx))),
            new_peer_rx: Arc::new(Mutex::new(Some(new_peer_rx))),
            send_udp_tx,
        }
    }
}
