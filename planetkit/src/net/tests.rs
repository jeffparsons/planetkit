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

// Network node helper. Contains all the network
// server bits and Specs bits required to simulate
// a network node, so we can easily play with multiple.
struct Node {
    server_port: u16,
    world: specs::World,
    dispatcher: specs::Dispatcher<'static, 'static>,
}

impl Node {
    pub fn new(log: &slog::Logger) -> Node {
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

        // Create sender system, receiver system,
        // and spawn network servers.
        let mut world = specs::World::new();
        let recv_system = RecvSystem::<TestMessage>::new(&log, &mut world);
        let mut send_system = SendSystem::<TestMessage>::new(&log, &mut world);
        let server_port = start_tcp_server(
            &log,
            recv_system.sender().clone(),
            send_system.take_new_peer_sender().expect("Somebody else took it!"),
            remote.clone(),
            None,
        );
        start_udp_server(
            &log,
            recv_system.sender().clone(),
            send_system.take_send_udp_wire_message_rx().expect("Somebody else took it!"),
            remote.clone(),
            server_port,
        );

        // TEMP/TODO: track actual peer addresses
        // For now, just send it to ourself.
        let connect_addr = format!("127.0.0.1:{}", server_port);
        let connect_addr: SocketAddr = connect_addr.parse().unwrap();
        send_system.set_one_true_peer_addr(connect_addr);

        // Make a dispatcher.
        let dispatcher = specs::DispatcherBuilder::new()
            .add(recv_system, "recv", &[])
            .add(send_system, "send", &[])
            .build();

        Node {
            server_port: server_port,
            world: world,
            dispatcher: dispatcher,
        }
    }

    pub fn dispatch(&mut self) {
        self.dispatcher.dispatch(&mut self.world.res);
    }
}

//
// TODO: Make this test higher-level, and use both TCP and UDP server together.
//
#[test]
fn all_the_way_from_send_system_to_recv_system() {
    let drain = slog::Discard;
    let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));

    let mut node1 = Node::new(&root_log);

    // Put a message on the SendMessageQueue.
    // (NLL SVP.)
    // TODO: do this from another node,
    // after having it connect to the first one.
    {
        let send_queue = &mut node1.world.write_resource::<SendMessageQueue<TestMessage>>().queue;
        send_queue.push_back(
            SendMessage {
                game_message: TestMessage{
                    disposition: "Sunny!".to_string()
                }
            }
        );
    }

    // Step the world.
    // This should send the message.
    node1.dispatch();
    // Sleep a while to make sure we receive the message.
    std::thread::sleep(Duration::from_millis(100));
    // This should receive the message.
    node1.dispatch();

    // Take a look at what ended up in the received message queue.
    let recv_queue = &mut node1.world.write_resource::<RecvMessageQueue<TestMessage>>().queue;
    // We should have received something equivalent to what we sent.
    assert_eq!(recv_queue.len(), 1);
    let expected_message = TestMessage {
        disposition: "Sunny!".to_string()
    };
    assert_eq!(recv_queue.pop_front().unwrap().game_message, expected_message);

    // TODO: gracefully shut down the server before the end of all tests;
    // you don't want to leave the thread hanging around awkwardly.
}
