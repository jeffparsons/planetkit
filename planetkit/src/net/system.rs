use std;
use std::io;
use std::net::SocketAddr;

use futures;
use tokio_core::net::{UdpSocket, UdpCodec};
use specs;
use slog::Logger;
use serde_json;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
enum Message {
    /// First message you should send to any peer when establishing a connection
    /// (keeping in mind that this is only a logical connection in PlanetKit, not a stateful TCP connection)
    /// regardless of the roles each peer might have (server, client, equal).
    Hello,
    /// Courtesy message before disconnecting, so that your peer can regard
    /// you as having cleanly disconnected rather than mysteriously disappearing.
    Goodbye,
    // TODO: flesh out
    BadMessage,
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
        serde_json::from_slice(buf).or_else(|error| {
            // TODO: don't warn here; trace here with details unless we can wrap
            // them up in an error to log below.
            warn!(self.log, "Got a bad message from peer"; "peer_addr" => format!("{:?}", src), "message" => format!("{:?}", buf), "error" => format!("{:?}", error));
            Ok(Message::BadMessage)
        })
    }

    fn encode(&mut self, _message: Message, _buf: &mut Vec<u8>) -> SocketAddr {
        panic!("Not implemented");
    }
}

pub struct System {
    log: Logger,
    inbound_message_rx: std::sync::mpsc::Receiver<Message>,
}

// TODO: take a parameter for game-specific message type.
// TODO: accept should_listen parameter and not listen otherwise.
impl System {
    pub fn new(parent_log: &Logger) -> System {
        use std::thread;
        use tokio_core::reactor::Core;
        use futures::Stream;

        // Create an unbounded channel. We'll make it `Codec`'s job to record
        // message rates/sizes from each peer, and to start rejecting them if
        // the peer is too chatty, or was never allowed to connect in the first place.
        //
        // We can't do this in the `System` because there could be thousands of
        // messages coming in between ticks.
        let (inbound_message_tx, inbound_message_rx) = std::sync::mpsc::channel();

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
            let f = stream.filter(|message| {
                // TODO: log
                match message {
                    &Message::BadMessage => {
                        println!("Got a bad message from peer: {:?}", message);
                        false
                    },
                    _ => true,
                }
            }).for_each(move |message| {
                // TODO: Only do this at debug level for a while, then demote to trace.
                info!(server_log, "Got message"; "message" => format!("{:?}", message));

                // Send the message to net System.
                //
                // TODO: how are we going to dispatch messages to the systems that
                // need to know about them? Input adapter just sends it to _all_ systems,
                // but that's probably not going to fly. There needs to be a central
                // dispatcher that can decide based on message type, that you provide
                // to the network system.
                //
                // Individual systems shouldn't need to know about whatever the
                // wrapper type for the specific game is. So individual systems
                // might need a reference to the "dispatcher", whatever it is.
                // Maybe it's kind of like a codec object?
                inbound_message_tx.send(message).expect("Receiver hung up?");

                futures::future::ok(())
            });
            // TODO: handle error; log warning, don't crash server.
            // (The stream will terminate on first error.)
            // Or maybe do all the handling in `Codec`.

            reactor.run(f).expect("Server reactor failed");
        }).expect("Failed to spawn server thread");

        System {
            log: parent_log.new(o!()),
            inbound_message_rx: inbound_message_rx,
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

#[cfg(test)]
mod tests {
    use futures::Future;
    use tokio_core::reactor::Core;
    use slog;

    use super::*;

    #[test]
    fn receive_corrupt_message() {
        // Receiving a corrupt message should not kill the reactor.
        let drain = slog::Discard;
        let log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));

        let system = System::new(&log);

        // Bind socket for sending message.
        // TODO: pick a random / free port.
        let addr = "0.0.0.0:62832".to_string();
        let addr = addr.parse::<SocketAddr>().unwrap();
        let mut reactor = Core::new().expect("Failed to create reactor");
        let handle = reactor.handle();
        let socket = UdpSocket::bind(&addr, &handle).expect("Failed to bind socket");

        // Send a dodgy message.
        let target_addr = "127.0.0.1:62831".to_string();
        let target_addr = target_addr.parse::<SocketAddr>().unwrap();
        // Oops, it's lowercase; it won't match any message type!
        let f = socket.send_dgram(b"\"hello\"", target_addr).and_then(move |(socket2, _buf)| {
            // TODO: sleep; delivery order isn't guaranteed, even though
            // it almost certainly will be fine on localhost. (TODO: look this up.)
            socket2.send_dgram(b"\"Goodbye\"", target_addr)
        });
        reactor.run(f).expect("Test reactor failed");

        let message = system.inbound_message_rx.recv().expect("Failed to receive message");
        assert_eq!(message, Message::Goodbye);

        // TODO: gracefully shut down the server before the end of all tests;
        // you don't want to leave the thread hanging around awkwardly.
    }
}
