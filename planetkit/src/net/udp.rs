use std;
use std::result::Result;
use std::io;
use std::net::SocketAddr;
use std::sync::mpsc;

use futures::{self, sync};
use tokio_core::reactor::Remote;
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

    fn encode(&mut self, message: SendWireMessage<G>, buf: &mut Vec<u8>) -> SocketAddr {
        serde_json::to_writer(buf, &message.message).expect("Error encoding message");
        message.dest
    }
}

// Forwards messages to the `RecvSystem`. Leaves the first-pass (by host, not peer ID)
// rate-limiting etc. to the `Codec`, because there could be thousands of messages
// received per second, and the `RecvSystem` buffers up messages for a while before
// getting to them.
//
// Listens on all network interfaces.
// Picks a random port if none was specified.
//
// Returns the actual port that was bound.
//
// TODO: mechanism to stop server.
pub fn start_udp_server<G: GameMessage, MaybePort>(
    parent_log: &Logger,
    recv_system_sender: mpsc::Sender<RecvWireMessage<G>>,
    send_system_udp_receiver: sync::mpsc::Receiver<SendWireMessage<G>>,
    remote: Remote,
    port: MaybePort
) -> u16
    where MaybePort: Into<Option<u16>>
{
    use futures::{Future, Stream, Sink};

    // Don't return to caller until we've bound the socket,
    // or we might miss some messages.
    // (This came up in tests that talk to localhost.)
    // Also use this to communicate the actual address we bound to.
    let (actual_port_tx, actual_port_rx) = std::sync::mpsc::channel::<u16>();

    // Pick a random port if none was specified.
    let addr = format!("0.0.0.0:{}", port.into().unwrap_or(0));
    let addr = addr.parse::<SocketAddr>().unwrap();

    // Run reactor on its own thread so we can always be receiving messages
    // from peers, and buffer them up until we're ready to process them.
    let server_log = parent_log.new(o!());
    let server_error_log = server_log.new(o!());
    let sink_error_log = server_log.new(o!());
    let codec_log = parent_log.new(o!());

    remote.spawn(move |handle| {
        let socket = UdpSocket::bind(&addr, &handle).expect("Failed to bind server socket");
        let actual_addr = socket.local_addr().expect("Socket isn't bound");

        info!(server_log, "UDP server listening"; "addr" => format!("{}", actual_addr));

        // Let main thread know we're ready to receive messages.
        actual_port_tx.send(actual_addr.port()).expect("Receiver hung up");

        let codec = Codec::<G>{
            log: codec_log,
            _phantom_game_message: std::marker::PhantomData,
        };
        let (sink, stream) = socket.framed(codec).split();

        // Sender future
        let sink = sink.sink_map_err(move |err| {
            error!(sink_error_log, "Unexpected error in sending to sink"; "err" => format!("{}", err));
            ()
        });
        // Throw away the source and sink when we're done; what else do we want with them? :)
        let tx_f = sink.send_all(send_system_udp_receiver).map(|_| ());
        handle.spawn(tx_f);

        // Receiver future
        let rx_f = stream
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
            }).or_else(move |error| {
                info!(server_error_log, "Something broke in listening for connections"; "error" => format!("{}", error));
                futures::future::ok(())
            });

        rx_f
    });

    // Wait until socket is bound before telling the caller what address we bound.
    actual_port_rx.recv().expect("Sender hung up")
}

#[cfg(test)]
mod tests {
    use super::*;

    use std;
    use std::thread;
    use std::time::Duration;

    use futures::Future;
    use tokio_core::reactor::{Core, Timeout};
    use tokio_core::net::UdpSocket;
    use slog;

    // Nothing interesting in here!
    #[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
    struct TestMessage {}
    impl GameMessage for TestMessage{}

    #[test]
    fn receive_corrupt_message() {
        // Receiving a corrupt message should not kill the reactor.

        // Run reactor on its own thread.
        let (remote_tx, remote_rx) = mpsc::channel::<Remote>();
        thread::Builder::new()
            .name("tcp_server".to_string())
            .spawn(move || {
                let mut reactor = Core::new().expect("Failed to create reactor for network server");
                remote_tx.send(reactor.remote()).expect("Receiver hung up");
                reactor.run(futures::future::empty::<(), ()>()).expect("Network server reactor failed");
            }).expect("Failed to spawn server thread");
        let remote = remote_rx.recv().expect("Sender hung up");

        // Spawn network server on other thread.
        let drain = slog::Discard;
        let log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));
        let (recv_tx, recv_rx) = mpsc::channel::<RecvWireMessage<TestMessage>>();
        // Tiny buffer is fine for test. Someone else can figure out how
        // big is reasonable in the real world.
        let (_send_tx, send_rx) = sync::mpsc::channel::<SendWireMessage<TestMessage>>(10);
        let server_port = start_udp_server(&log, recv_tx, send_rx, remote, None);

        // Bind socket for sending message.
        let addr = "0.0.0.0:0".to_string();
        let addr = addr.parse::<SocketAddr>().unwrap();
        let mut reactor = Core::new().expect("Failed to create reactor");
        let handle = reactor.handle();
        let socket = UdpSocket::bind(&addr, &handle).expect("Failed to bind socket");

        // Send a dodgy message.
        // Oops, it's lowercase; it won't match any message type!
        let dest_addr = format!("127.0.0.1:{}", server_port);
        let dest_addr: SocketAddr = dest_addr.parse().unwrap();
        let f = socket.send_dgram(b"\"hello\"", dest_addr).and_then(
            |(socket2, _buf)| {
                // Wait a bit; delivery order isn't guaranteed,
                // even though it will almost certainly be fine on localhost.
                Timeout::new(Duration::from_millis(10), &handle).expect("Failed to set timeout").and_then(
                    move |_| {
                        socket2.send_dgram(b"{\"Game\":{}}", dest_addr)
                    }
                )
            },
        );
        reactor.run(f).expect("Test reactor failed");

        // Sleep a while to make sure we receive the message.
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Take a look at what was received. Only one message should have made
        // it through to the RecvSystem channel.
        let recv_wire_message = recv_rx.recv().expect("Should have been something on the channel");
        assert_eq!(recv_wire_message.message, Ok(WireMessage::Game(TestMessage{})));
        // There shouldn't be any more messages on the channel.
        assert_eq!(recv_rx.try_recv(), Err(mpsc::TryRecvError::Empty));

        // TODO: gracefully shut down the server before the end of all tests;
        // you don't want to leave the thread hanging around awkwardly.
    }
}
