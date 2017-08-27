use std;
use std::result::Result;
use std::io;
use std::net::SocketAddr;
use std::sync::mpsc;

use bytes::BytesMut;
use futures;
use tokio_core::net::TcpListener;
use tokio_io::codec::{Encoder, Decoder};
use slog::Logger;
use serde_json;

use super::{GameMessage, WireMessage, SendWireMessage, RecvWireMessage};

struct Codec<G> {
    peer_addr: SocketAddr,
    _log: Logger,
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
        // TODO: identify the peer from a list of connected peers.
        // Or... is that the role of the server logic below? Probably the server
        // logic below... because it allows us to store less state up here.
        // Which means that RecvWireMessage contains source address. That sounds right.

        // Now that we have both TCP and UDP servers we should try to keep as much as
        // possible off in common code somewhere.
        serde_json::from_slice::<WireMessage<G>>(buf)
        .map(|message| {
            Some(RecvWireMessage {
                src: self.peer_addr,
                message: Result::Ok(message)
            })
        })
        .or_else(|_error| {
            // TODO: how to tell if it's a partial message?
            // Can I get an error out of Serde to say "it could have maybe
            // parsed right if there was more there"?
            Ok(None)
        })
    }
}

pub fn start_tcp_server<G: GameMessage>(parent_log: &Logger, recv_system_sender: mpsc::Sender<RecvWireMessage<G>>) {
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
        .name("tcp_server".to_string())
        .spawn(move || {
            let mut reactor = Core::new().expect("Failed to create reactor for network server");
            let handle = reactor.handle();
            let socket = TcpListener::bind(&addr, &handle).expect("Failed to bind server socket");

            info!(server_log, "TCP server listening"; "addr" => format!("{}", addr));

            // Let main thread know we're ready to receive messages.
            ready_tx.send(()).expect("Receiver hung up");

            let f = socket.incoming().for_each(move |(socket, peer_addr)| {
                use tokio_io::AsyncRead;
                let codec = Codec::<G>{
                    peer_addr: peer_addr,
                    _log: codec_log.clone(),
                    _phantom_game_message: std::marker::PhantomData,
                };
                let stream = socket.framed(codec);
                let peer_recv_system_sender = recv_system_sender.clone();
                let peer_server_log = server_log.new(o!("peer_addr" => format!("{}", peer_addr)));
                stream.filter(|recv_wire_message| {
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
                    peer_recv_system_sender.send(recv_wire_message).expect("Receiver hung up?");

                    futures::future::ok(())
                })
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

// TODO: TESTS!