use std::io;
use std::net::SocketAddr;

use futures::{Future, Poll};
use tokio_core::net::UdpSocket;
use specs;
use slog::Logger;

struct Server {
    socket: UdpSocket,
    buf: Vec<u8>,
    log: Logger,
}

impl Future for Server {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        loop {
            let (bytes_received, _socket_addr) = try_nb!(self.socket.recv_from(&mut self.buf));
            info!(self.log, "Received bytes from peer"; "bytes" => bytes_received);
        }
    }
}

pub struct System {
    log: Logger,
}

// TODO: take a parameter for game-specific message type.
// TODO: accept should_listen parameter and not listen otherwise.
impl System {
    pub fn new(parent_log: &Logger) -> System {
        use std::thread;
        use tokio_core::reactor::Core;

        let addr = "0.0.0.0:62831".to_string();
        let addr = addr.parse::<SocketAddr>().unwrap();

        // Run reactor on its own thread so we can always be receiving messages
        // from peers, and buffer them up until we're ready to process them.
        let server_log = parent_log.new(o!());
        thread::Builder::new().name("server".to_string()).spawn(move || {
            let mut reactor = Core::new().expect("Failed to create reactor for System");
            let handle = reactor.handle();
            let socket = UdpSocket::bind(&addr, &handle).expect("Failed to bind server socket");
            info!(server_log, "Listening"; "addr" => format!("{}", addr));

            let server = Server {
                log: server_log,
                socket: socket,
                buf: vec![0; 1024],
            };
            reactor.run(server).expect("Server reactor failed");
        }).expect("Failed to spawn server thread");

        System {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for System {
    type SystemData = (
    );

    fn run(&mut self, data: Self::SystemData) {
        // ...
    }
}
