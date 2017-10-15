use std;
use std::thread;
use std::net::SocketAddr;

use slog;
use futures;
use tokio_core::reactor::{Core, Remote};

use super::{
    SendWireMessage,
    RecvWireMessage,
    NewPeer,
    GameMessage,
};

/// Network client/server.
///
/// Makes connections over TCP, and sends/listens
/// on UDP.
pub struct Server<G> {
    remote: Remote,
    log: slog::Logger,
    recv_system_sender: std::sync::mpsc::Sender<RecvWireMessage<G>>,
    send_system_new_peer_sender: std::sync::mpsc::Sender<NewPeer<G>>,
    // Only exists until used to start UDP server.
    send_udp_wire_message_rx: Option<futures::sync::mpsc::Receiver<SendWireMessage<G>>>,
    // Server port, if listening.
    // TODO: put into a ServerState enum or something.
    pub port: Option<u16>,
}

impl<G: GameMessage> Server<G> {
    // TODO: require deciding up-front whether to listen on TCP,
    // or be a "pure client"?
    pub fn new(
        parent_logger: &slog::Logger,
        recv_system_sender: std::sync::mpsc::Sender<RecvWireMessage<G>>,
        send_system_new_peer_sender: std::sync::mpsc::Sender<NewPeer<G>>,
        send_udp_wire_message_rx: futures::sync::mpsc::Receiver<SendWireMessage<G>>,
    ) -> Server<G> {
        // Run reactor on its own thread.
        let (remote_tx, remote_rx) = std::sync::mpsc::channel::<Remote>();
        thread::Builder::new()
            .name("network_server".to_string())
            .spawn(move || {
                let mut reactor = Core::new().expect("Failed to create reactor for network server");
                remote_tx.send(reactor.remote()).expect("Receiver hung up");
                reactor.run(futures::future::empty::<(), ()>()).expect("Network server reactor failed");
            }).expect("Failed to spawn server thread");
        let remote = remote_rx.recv().expect("Sender hung up");

        Server {
            remote: remote,
            log: parent_logger.new(o!()),
            recv_system_sender: recv_system_sender,
            send_system_new_peer_sender: send_system_new_peer_sender,
            send_udp_wire_message_rx: Some(send_udp_wire_message_rx),
            port: None,
        }
    }

    pub fn start_listen<MaybePort>(
        &mut self,
        port: MaybePort,
    )
        where MaybePort: Into<Option<u16>>
    {
        self.port = super::tcp::start_tcp_server(
            &self.log,
            self.recv_system_sender.clone(),
            self.send_system_new_peer_sender.clone(),
            self.remote.clone(),
            port,
        ).into();
        super::udp::start_udp_server(
            &self.log,
            self.recv_system_sender.clone(),
            self.send_udp_wire_message_rx.take().expect("Somebody else took it!"),
            self.remote.clone(),
            self.port,
        );
    }

    pub fn connect(&mut self, addr: SocketAddr) {
        super::tcp::connect_to_server(
            &self.log,
            self.recv_system_sender.clone(),
            self.send_system_new_peer_sender.clone(),
            self.remote.clone(),
            addr,
        );
        // TODO: listen on UDP using the same port
        // that TCP server bound. We'll need this
        // for communicating with the server!!!
    }
}
