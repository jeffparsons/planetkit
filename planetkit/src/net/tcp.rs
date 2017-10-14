use std;
use std::result::Result;
use std::io;
use std::net::SocketAddr;
use std::mem::size_of;

use bytes::{BytesMut, BigEndian, ByteOrder};
use futures::{self, Future};
use tokio_core::reactor::{Remote, Handle};
use tokio_core::net::{TcpListener, TcpStream};
use tokio_io::codec::{Encoder, Decoder};
use slog::Logger;
use serde_json;

use super::{
    GameMessage,
    WireMessage,
    SendWireMessage,
    RecvWireMessage,
    NewPeer,
};

type MessageLengthPrefix = u16;

struct Codec<G> {
    peer_addr: SocketAddr,
    log: Logger,
    _phantom_game_message: std::marker::PhantomData<G>,
}

impl<G: GameMessage> Encoder for Codec<G> {
    type Item = SendWireMessage<G>;
    type Error = io::Error;

    fn encode(&mut self, _message: SendWireMessage<G>, _buf: &mut BytesMut) -> Result<(), io::Error> {
        panic!("Not implemented");
    }
}

impl<G: GameMessage> Decoder for Codec<G> {
    type Item = RecvWireMessage<G>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<RecvWireMessage<G>>, io::Error> {
        // Keep waiting if we haven't received a message header.
        if buf.len() < size_of::<MessageLengthPrefix>() {
            return Ok(None);
        }

        // Keep waiting if we haven't received at least one whole message.
        let message_length = BigEndian::read_u16(buf) as usize;
        if buf.len() < size_of::<MessageLengthPrefix>() + message_length {
            return Ok(None);
        }

        // TODO: identify the peer from a list of connected peers.
        // Or... is that the role of the server logic below? Probably the server
        // logic below... because it allows us to store less state up here.
        // Which means that RecvWireMessage contains source address. That sounds right.

        // Ok, we should have at least one whole message in our buffer.
        // Skip the length prefix, and try to parse the message.
        buf.split_to(size_of::<MessageLengthPrefix>());
        serde_json::from_slice::<WireMessage<G>>(&buf[0..message_length])
        .map(|message| {
            // Advance the buffer past the message we found.
            buf.split_to(message_length);
            Some(RecvWireMessage {
                src: self.peer_addr,
                message: Result::Ok(message)
            })
        })
        .map_err(|error| {
            warn!(
                self.log,
                "Got a bad message from peer";
                "peer_addr" => format!("{:?}", self.peer_addr),
                "message_length" => message_length,
                "buffer" => format!("{:?}", buf),
                "error" => format!("{:?}", error)
            );
            io::Error::new(io::ErrorKind::Other, "Couldn't parse message")
        })
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
pub fn start_tcp_server<G: GameMessage, MaybePort>(
    parent_log: &Logger,
    recv_system_sender: std::sync::mpsc::Sender<RecvWireMessage<G>>,
    // Used to establish new peer connections,
    // and register the sender ends of channels
    // to send messages to those connections.
    send_system_new_peer_sender:
        std::sync::mpsc::Sender<NewPeer<G>>,
    remote: Remote,
    port: MaybePort
) -> u16
    where MaybePort: Into<Option<u16>>
{
    use futures::Stream;

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

    remote.spawn(move |handle| {
        let socket = TcpListener::bind(&addr, &handle).expect("Failed to bind server socket");
        let actual_addr = socket.local_addr().expect("Socket isn't bound");

        info!(server_log, "TCP server listening"; "addr" => format!("{}", actual_addr));

        // Let main thread know we're ready to receive messages.
        actual_port_tx.send(actual_addr.port()).expect("Receiver hung up");

        let cloned_handle = handle.clone();
        let f = socket.incoming().for_each(move |(socket, peer_addr)| {
            handle_tcp_stream(
                &cloned_handle,
                socket,
                peer_addr,
                &server_log,
                recv_system_sender.clone(),
                send_system_new_peer_sender.clone(),
            )
        }).or_else(move |error| {
            info!(server_error_log, "Something broke in listening for connections"; "error" => format!("{}", error));
            futures::future::ok(())
        });

        // TODO: handle stream disconnection somewhere.
        // (The stream will terminate on first error.)

        f
    });

    // Wait until socket is bound before telling the caller what port we bound.
    actual_port_rx.recv().expect("Sender hung up")
}

pub fn connect_to_server<G: GameMessage>(
    parent_log: &Logger,
    recv_system_sender: std::sync::mpsc::Sender<RecvWireMessage<G>>,
    // Used to establish new peer connections,
    // and register the sender ends of channels
    // to send messages to those connections.
    send_system_new_peer_sender:
        std::sync::mpsc::Sender<NewPeer<G>>,
    remote: Remote,
    addr: SocketAddr,
) {
    // Don't return until we've actually established a connection,
    // or we might miss some messages.
    let (connection_established_tx, connection_established_rx) = std::sync::mpsc::channel::<()>();

    // Run reactor on its own thread so we can always be receiving messages
    // from peers, and buffer them up until we're ready to process them.
    let client_log = parent_log.new(o!());
    let client_error_log = client_log.new(o!());

    remote.spawn(move |handle| {
        let socket_future = TcpStream::connect(&addr, &handle);

        let cloned_handle = handle.clone();
        let f = socket_future.and_then(move |socket| {
            connection_established_tx.send(()).expect("Receiver hung up?");
            handle_tcp_stream(
                &cloned_handle,
                socket,
                addr,
                &client_log,
                recv_system_sender,
                send_system_new_peer_sender,
            )
        }).or_else(move |error| {
            // TODO: figure out more specific error; decide where each is handled.
            info!(client_error_log, "Something broke in connecting to server, or handling connection"; "error" => format!("{}", error));
            futures::future::ok(())
        });

        f
    });

    // Wait until connection is established.
    connection_established_rx.recv().expect("Sender hung up")
}

// Handle sending/receiving and encoding/decoding messages
// once a TCP stream (as either client or server) has been
// established.
fn handle_tcp_stream<G: GameMessage>(
    handle: &Handle,
    socket: TcpStream,
    peer_addr: SocketAddr,
    parent_log: &Logger,
    recv_system_sender: std::sync::mpsc::Sender<RecvWireMessage<G>>,
    // Used to establish new peer connections,
    // and register the sender ends of channels
    // to send messages to those connections.
    send_system_new_peer_sender: std::sync::mpsc::Sender<NewPeer<G>>,
) -> Box<Future<Item=(), Error=std::io::Error>> {
    use futures::Stream;
    use futures::Sink;
    use tokio_io::AsyncRead;

    let codec = Codec::<G>{
        peer_addr: peer_addr,
        log: parent_log.new(o!()),
        _phantom_game_message: std::marker::PhantomData,
    };
    let (sink, stream) = socket.framed(codec).split();

    // Sender future
    let sink_error_log = parent_log.new(o!("peer_addr" => format!("{}", peer_addr)));
    let sink = sink.sink_map_err(move |err| {
        error!(sink_error_log, "Unexpected error in sending to sink"; "err" => format!("{}", err));
        ()
    });
    // Create a channel for the SendSystem to
    // send messages over this TCP connection,
    // and use it to notify the SendSystem that
    // we've connected with a new peer.
    // TODO: how big is reasonable here? Unbounded? Probably...
    let (tcp_tx, tcp_rx) = futures::sync::mpsc::channel::<SendWireMessage<G>>(1000);
    let new_peer = NewPeer {
        tcp_sender: tcp_tx,
        socket_addr: peer_addr,
    };
    send_system_new_peer_sender.send(new_peer).expect("Receiver hung up?");
    // Throw away the source and sink when we're done; what else do we want with them? :)
    let tx_f = sink.send_all(tcp_rx).map(|_| ());
    handle.spawn(tx_f);

    // Receiver future
    let peer_server_log = parent_log.new(o!("peer_addr" => format!("{}", peer_addr)));
    let peer_server_error_log = peer_server_log.clone();
    let f = stream.filter(|recv_wire_message| {
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
        info!(peer_server_log, "Got recv_wire_message"; "recv_wire_message" => format!("{:?}", recv_wire_message));

        // Send the message to net RecvSystem, to be interpreted and dispatched.
        recv_system_sender.send(recv_wire_message).expect("Receiver hung up?");

        futures::future::ok(())
    }).or_else(move |error| {
        // Got a bad message from the peer (I assume) so the
        // connection is going to close.
        info!(peer_server_error_log, "Peer broke pipe"; "error" => format!("{}", error));
        futures::future::ok(())
    });
    Box::new(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std;
    use std::thread;

    use futures::{self, Future};
    use tokio_core::reactor::Core;
    use tokio_core::net::TcpStream;
    use tokio_io::io::write_all;
    use slog;
    use bytes::BufMut;

    // Nothing interesting in here!
    #[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
    struct TestMessage {}
    impl GameMessage for TestMessage{}

    #[test]
    fn receive_corrupt_message() {
        // Receiving a corrupt message should not kill the reactor.

        // Run reactor on its own thread.
        let (remote_tx, remote_rx) = std::sync::mpsc::channel::<Remote>();
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
        let (tx, rx) = std::sync::mpsc::channel::<RecvWireMessage<TestMessage>>();
        let (new_peer_tx, _new_peer_rx) = std::sync::mpsc::channel::<NewPeer<TestMessage>>();
        let server_port = start_tcp_server(&log, tx, new_peer_tx, remote, None);

        // Connect to server.
        let connect_addr = format!("127.0.0.1:{}", server_port);
        let connect_addr: SocketAddr = connect_addr.parse().unwrap();

        let mut reactor = Core::new().expect("Failed to create reactor");
        let handle = reactor.handle();
        let socket_future = TcpStream::connect(&connect_addr, &handle);

        // Send a dodgy message.
        // Oops, it's lowercase; it won't match any message type!
        let mut buf = BytesMut::with_capacity(1000);
        let mut buf2 = BytesMut::with_capacity(1000);
        let f = socket_future.and_then(|tcp_stream| {
            let message = b"\"hello\"";
            buf.put_u16::<BigEndian>(message.len() as u16);
            buf.put_slice(message);
            write_all(tcp_stream, &mut buf)
        }).and_then(|stream_and_buffer| {
            let tcp_stream = stream_and_buffer.0;
            let message = b"{\"Game\":{}}";
            buf2.put_u16::<BigEndian>(message.len() as u16);
            buf2.put_slice(message);
            write_all(tcp_stream, &mut buf2)
        });

        reactor.run(f).expect("Test reactor failed");

        // Sleep a while to make sure we receive the message.
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Take a look at what was received. The bad message should have terminated the connection,
        // so nothing should have made it through to the game message channel.
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // TODO: gracefully shut down the server before the end of all tests;
        // you don't want to leave the thread hanging around awkwardly.
    }

    #[test]
    fn receive_two_messages_in_one_segment() {
        // Receiving two message in one segment (probably) should result
        // in both being happily parsed and forwarded to game message channel.

        // Run reactor on its own thread.
        let (remote_tx, remote_rx) = std::sync::mpsc::channel::<Remote>();
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
        let (tx, rx) = std::sync::mpsc::channel::<RecvWireMessage<TestMessage>>();
        let (new_peer_tx, _new_peer_rx) = std::sync::mpsc::channel::<NewPeer<TestMessage>>();
        let server_port = start_tcp_server(&log, tx, new_peer_tx, remote, None);

        // Connect to server.
        let connect_addr = format!("127.0.0.1:{}", server_port);
        let connect_addr: SocketAddr = connect_addr.parse().unwrap();
        let mut reactor = Core::new().expect("Failed to create reactor");
        let handle = reactor.handle();
        let socket_future = TcpStream::connect(&connect_addr, &handle);

        // Send to great messages together!
        let mut buf = BytesMut::with_capacity(1000);
        let f = socket_future.and_then(|tcp_stream| {
            let message = b"{\"Game\":{}}";
            // Put the message twice in a row.
            buf.put_u16::<BigEndian>(message.len() as u16);
            buf.put_slice(message);
            buf.put_u16::<BigEndian>(message.len() as u16);
            buf.put_slice(message);

            // Write the whole thing to the TCP stream.
            write_all(tcp_stream, &mut buf)
        });

        reactor.run(f).expect("Test reactor failed");

        // Sleep a while to make sure we receive the message.
        let blink = std::time::Duration::from_millis(100);
        std::thread::sleep(blink);

        // Take a look at what was received. We should have gotten two
        // identical `TestMessage`s.
        let recv_wire_message = rx.recv_timeout(blink).expect("Should have found our first message on the channel");
        assert_eq!(recv_wire_message.message, Ok(WireMessage::Game(TestMessage{})));
        let recv_wire_message = rx.recv_timeout(blink).expect("Should have found our second message on the channel");
        assert_eq!(recv_wire_message.message, Ok(WireMessage::Game(TestMessage{})));
        // There shouldn't be any more messages on the channel.
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // TODO: gracefully shut down the server before the end of all tests;
        // you don't want to leave the thread hanging around awkwardly.
    }
}
