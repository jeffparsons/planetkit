use std::io;
use std::net::SocketAddr;

use futures;
use tokio_core::net::{UdpSocket, UdpCodec};
use specs;
use slog::Logger;
use serde_json;

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    /// First message you should send to any peer when establishing a connection
    /// (keeping in mind that this is only a logical connection in PlanetKit, not a stateful TCP connection)
    /// regardless of the roles each peer might have (server, client, equal).
    Hello,
    /// Courtesy message before disconnecting, so that your peer can regard
    /// you as having cleanly disconnected rather than mysteriously disappearing.
    Goodbye,
}

struct Codec {
    log: Logger,
}

// TODO: Negotiate codec with server. Start with JSON to make inspecting the
// initial handshake easy, and then optionally move to a more efficient encoding
// (so we can always keep it in JSON for debugging).
impl UdpCodec for Codec {
    type In = Message;
    type Out = Message;

    fn decode(&mut self, src: &SocketAddr, buf: &[u8]) -> io::Result<Message> {
        serde_json::from_slice(buf).map_err(|error| {
            // TODO: don't warn here; trace here with details unless we can wrap
            // them up in an error to log below.
            warn!(self.log, "Got a bad message from peer"; "peer_addr" => format!("{:?}", src), "message" => format!("{:?}", buf), "error" => format!("{:?}", error));
            io::Error::new(io::ErrorKind::Other, error)
        })
    }

    fn encode(&mut self, _message: Message, _buf: &mut Vec<u8>) -> SocketAddr {
        panic!("Not implemented");
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
        use futures::Stream;

        let addr = "0.0.0.0:62831".to_string();
        let addr = addr.parse::<SocketAddr>().unwrap();

        // Run reactor on its own thread so we can always be receiving messages
        // from peers, and buffer them up until we're ready to process them.
        let server_log = parent_log.new(o!());
        let codec_log = parent_log.new(o!());
        thread::Builder::new().name("server".to_string()).spawn(move || {
            let mut reactor = Core::new().expect("Failed to create reactor for System");
            let handle = reactor.handle();
            let socket = UdpSocket::bind(&addr, &handle).expect("Failed to bind server socket");
            info!(server_log, "Listening"; "addr" => format!("{}", addr));

            let codec = Codec {
                log: codec_log,
            };
            let stream = socket.framed(codec);
            let f = stream.for_each(move |message| {
                // TODO: Only do this at debug level for a while, then demote to trace.
                info!(server_log, "Got message"; "message" => format!("{:?}", message));
                futures::future::ok(())
            });
            // TODO: handle error; log warning, don't crash server.

            reactor.run(f).expect("Server reactor failed");
        }).expect("Failed to spawn server thread");

        System {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for System {
    type SystemData = (
    );

    fn run(&mut self, _data: Self::SystemData) {
        // ...
    }
}
