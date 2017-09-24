use std;
use std::time::Duration;
use std::sync::mpsc;
use std::thread;

use futures;
use tokio_core::reactor::{Core, Remote};
use slog;
use specs;

use super::*;

// Nothing interesting in here!
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct TestMessage {
    disposition: String,
}
impl GameMessage for TestMessage{}

//
// TODO: Make this test higher-level, and use both TCP and UDP server together.
//
#[test]
fn all_the_way_from_send_system_to_recv_system() {
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

    // Create sender system, receiver system, and spawn network server.
    let drain = slog::Discard;
    let log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));
    let mut world = specs::World::new();
    let recv_system = RecvSystem::<TestMessage>::new(&log, &mut world);
    let (mut send_system, send_rx) = SendSystem::<TestMessage>::new(&log, &mut world);
    let server_addr = start_udp_server(&log, recv_system.sender().clone(), send_rx, remote, None);
    // TEMP/TODO: track actual peer addresses
    // For now, just send it to ourself.
    send_system.set_one_true_peer_addr(server_addr);

    // Put a message on the SendMessageQueue.
    // NLL SVP.
    {
        let send_queue = &mut world.write_resource::<SendMessageQueue<TestMessage>>().queue;
        send_queue.push_back(
            SendMessage {
                game_message: TestMessage{
                    disposition: "Sunny!".to_string()
                }
            }
        );
    }

    // Make a dispatcher and step the world.
    let mut dispatcher = specs::DispatcherBuilder::new()
        .add(recv_system, "recv", &[])
        .add(send_system, "send", &[])
        .build();
    // This should send the message.
    dispatcher.dispatch(&mut world.res);
    // Sleep a while to make sure we receive the message.
    std::thread::sleep(Duration::from_millis(100));
    // This should receive the message.
    dispatcher.dispatch(&mut world.res);

    // Take a look at what ended up in the received message queue.
    let recv_queue = &mut world.write_resource::<RecvMessageQueue<TestMessage>>().queue;
    // We should have received something equivalent to what we sent.
    assert_eq!(recv_queue.len(), 1);
    let expected_message = TestMessage {
        disposition: "Sunny!".to_string()
    };
    assert_eq!(recv_queue.pop_front().unwrap().game_message, expected_message);

    // TODO: gracefully shut down the server before the end of all tests;
    // you don't want to leave the thread hanging around awkwardly.
}
