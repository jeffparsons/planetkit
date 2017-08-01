use std;
use std::result::Result;
use std::io;
use std::net::SocketAddr;
use std::sync::mpsc;

use futures;
use tokio_core::net::{UdpSocket, UdpCodec};
use slog::Logger;
use serde_json;

use super::{GameMessage, WireMessage, SendWireMessage, RecvWireMessage};

struct Codec<G> {
    log: Logger,
    _phantom_game_message: std::marker::PhantomData<G>,
}

// TODO: Negotiate protocol with server. Start with JSON to make inspecting the
// initial handshake easy, and then optionally move to a more efficient encoding
// (so we can always keep it in JSON for debugging).
//
// TODO: Think about how much information Code needs to store about each peer,
// e.g., their current expected encoding.
impl<G: GameMessage> UdpCodec for Codec<G> {
    type In = RecvWireMessage<G>;
    type Out = SendWireMessage<G>;

    fn decode(&mut self, src: &SocketAddr, buf: &[u8]) -> io::Result<RecvWireMessage<G>> {
        // TODO: identify the peer from a list of connected peers.
        serde_json::from_slice::<WireMessage<G>>(buf)
        .map(|message| {
            RecvWireMessage {
                src: *src,
                message: Result::Ok(message)
            }
        })
        .or_else(|error| {
            // TODO: don't warn here; trace here with details unless we can wrap
            // them up in an error to log below.
            warn!(self.log, "Got a bad message from peer"; "peer_addr" => format!("{:?}", src), "message" => format!("{:?}", buf), "error" => format!("{:?}", error));
            Ok(RecvWireMessage {
                src: *src,
                message: Result::Err(())
            })
        })
    }

    fn encode(&mut self, _message: SendWireMessage<G>, _buf: &mut Vec<u8>) -> SocketAddr {
        panic!("Not implemented");
    }
}

// Forwards messages to the `RecvSystem`. Leaves the first-pass (by host, not peer ID)
// rate-limiting etc. to the `Codec`, because there could be thousands of messages
// received per second, and the `RecvSystem` buffers up messages for a while before
// getting to them.
//
// TODO: mechanism to stop server.
pub fn start_server<G: GameMessage>(parent_log: &Logger, recv_system_sender: mpsc::Sender<RecvWireMessage<G>>) {
    use std::thread;
    use tokio_core::reactor::Core;
    use futures::Stream;

    // Don't return to caller until we've bound the socket,
    // or we might miss some messages.
    // (This came up in tests that talk to localhost.)
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<()>();

    let addr = "0.0.0.0:62831".to_string();
    let addr = addr.parse::<SocketAddr>().unwrap();

    // Run reactor on its own thread so we can always be receiving messages
    // from peers, and buffer them up until we're ready to process them.
    let server_log = parent_log.new(o!());
    let codec_log = parent_log.new(o!());
    thread::Builder::new()
        .name("server".to_string())
        .spawn(move || {
            let mut reactor = Core::new().expect("Failed to create reactor for network server");
            let handle = reactor.handle();
            let socket = UdpSocket::bind(&addr, &handle).expect("Failed to bind server socket");

            info!(server_log, "Listening"; "addr" => format!("{}", addr));

            // Let main thread know we're ready to receive messages.
            ready_tx.send(()).expect("Receiver hung up");

            let codec = Codec::<G>{
                log: codec_log,
                _phantom_game_message: std::marker::PhantomData,
            };
            let stream = socket.framed(codec);
            let f = stream
                .filter(|recv_wire_message| {
                    // TODO: log
                    match recv_wire_message.message {
                        Result::Err(_) => {
                            println!("Got a bad message from peer");
                            false
                        }
                        _ => true,
                    }
                })
                .for_each(move |recv_wire_message| {
                    // TODO: Only do this at debug level for a while, then demote to trace.
                    info!(server_log, "Got recv_wire_message"; "recv_wire_message" => format!("{:?}", recv_wire_message));

                    // Send the message to net RecvSystem, to be interpreted and dispatched.
                    recv_system_sender.send(recv_wire_message).expect("Receiver hung up?");

                    futures::future::ok(())
                });
            // TODO: handle error; log warning, don't crash server.
            // (The stream will terminate on first error.)
            // Or maybe do all the handling in `Codec`.

            reactor.run(f).expect("Server reactor failed");
        })
        .expect("Failed to spawn server thread");

    // Wait until socket is bound before returning system.
    ready_rx.recv().expect("Sender hung up");
}
