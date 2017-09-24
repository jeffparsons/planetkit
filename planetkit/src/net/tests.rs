use std;
use std::time::Duration;
use std::sync::mpsc;
use std::thread;

use futures::{self, Future};
use tokio_core::reactor::{Core, Remote, Timeout};
use tokio_core::net::UdpSocket;
use slog;
use specs;

use super::*;

// Nothing interesting in here!
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct TestMessage {}
impl GameMessage for TestMessage{}

//
// TODO: Make this test higher-level, and use both TCP and UDP server together.
//
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

    // Create receiver system and spawn network server.
    let drain = slog::Discard;
    let log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));
    let mut world = specs::World::new();
    let recv_system = RecvSystem::<TestMessage>::new(&log, &mut world);
    let server_addr = start_udp_server(&log, recv_system.sender().clone(), remote, None);

    // Bind socket for sending message.
    let addr = "0.0.0.0:0".to_string();
    let addr = addr.parse::<SocketAddr>().unwrap();
    let mut reactor = Core::new().expect("Failed to create reactor");
    let handle = reactor.handle();
    let socket = UdpSocket::bind(&addr, &handle).expect("Failed to bind socket");

    // Send a dodgy message.
    // Oops, it's lowercase; it won't match any message type!
    let f = socket.send_dgram(b"\"hello\"", server_addr).and_then(
        |(socket2, _buf)| {
            // Wait a bit; delivery order isn't guaranteed,
            // even though it will almost certainly be fine on localhost.
            Timeout::new(Duration::from_millis(10), &handle).expect("Failed to set timeout").and_then(
                move |_| {
                    socket2.send_dgram(b"{\"Game\":{}}", server_addr)
                }
            )
        },
    );
    reactor.run(f).expect("Test reactor failed");

    // Sleep a while to make sure we receive the message.
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Make a dispatcher and step the world.
    let mut dispatcher = specs::DispatcherBuilder::new()
        .add(recv_system, "recv", &[])
        .build();
    dispatcher.dispatch(&mut world.res);

    // Take a look at what ended up in the received message queue.
    let queue = &mut world.write_resource::<RecvMessageQueue<TestMessage>>().queue;
    // Only the good message should've made it through RecvSystem.
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.pop_front().unwrap().game_message, TestMessage{});

    // TODO: gracefully shut down the server before the end of all tests;
    // you don't want to leave the thread hanging around awkwardly.
}
