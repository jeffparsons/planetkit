use std;
use std::time::Duration;

use futures::Future;
use tokio_core::reactor::{Core, Timeout};
use tokio_core::net::UdpSocket;
use slog;
use specs;

use super::*;

// Nothing interesting in here!
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct TestMessage {}
impl GameMessage for TestMessage{}

#[test]
fn receive_corrupt_message() {
    // Receiving a corrupt message should not kill the reactor.
    let drain = slog::Discard;
    let log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));

    let mut world = specs::World::new();

    // Create receiver system and spawn network server.
    let recv_system = RecvSystem::<TestMessage>::new(&log, &mut world);
    start_server(&log, recv_system.sender().clone());

    // Bind socket for sending message.
    let addr = "0.0.0.0:0".to_string();
    let addr = addr.parse::<SocketAddr>().unwrap();
    let mut reactor = Core::new().expect("Failed to create reactor");
    let handle = reactor.handle();
    let socket = UdpSocket::bind(&addr, &handle).expect("Failed to bind socket");

    // Send a dodgy message.
    let target_addr = "127.0.0.1:62831".to_string();
    let target_addr = target_addr.parse::<SocketAddr>().unwrap();
    // Oops, it's lowercase; it won't match any message type!
    let f = socket.send_dgram(b"\"hello\"", target_addr).and_then(
        |(socket2, _buf)| {
            // Wait a bit; delivery order isn't guaranteed,
            // even though it will almost certainly be fine on localhost.
            Timeout::new(Duration::from_millis(10), &handle).expect("Failed to set timeout").and_then(
                move |_| {
                    socket2.send_dgram(b"{\"Game\":{}}", target_addr)
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
