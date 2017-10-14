use std;
use std::time::Duration;

use slog;
use specs;

use super::*;

// Nothing interesting in here!
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct TestMessage {
    disposition: String,
}
impl GameMessage for TestMessage {}

// Network node helper. Contains all the network
// server bits and Specs bits required to simulate
// a network node, so we can easily play with multiple.
struct Node {
    server: Server<TestMessage>,
    world: specs::World,
    dispatcher: specs::Dispatcher<'static, 'static>,
}

impl Node {
    pub fn new(log: &slog::Logger) -> Node {
        // Create sender and receiver systems.
        let mut world = specs::World::new();
        let recv_system = RecvSystem::<TestMessage>::new(&log, &mut world);
        let mut send_system = SendSystem::<TestMessage>::new(&log, &mut world);

        // Spawn TCP and UDP server/client.
        let mut server = Server::new(
            &log,
            recv_system.sender().clone(),
            send_system.take_new_peer_sender().expect("Somebody else took it!"),
            send_system.take_send_udp_wire_message_rx().expect("Somebody else took it!"),
        );
        server.start_listen();

        // Make a dispatcher.
        let dispatcher = specs::DispatcherBuilder::new()
            .add(recv_system, "recv", &[])
            .add(send_system, "send", &[])
            .build();

        Node {
            server: server,
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

    let mut sender_node = Node::new(&root_log);
    let mut receiver_node = Node::new(&root_log);

    let connect_addr = format!("127.0.0.1:{}", receiver_node.server.port.expect("Should be listening"));
    let connect_addr: SocketAddr = connect_addr.parse().unwrap();
    sender_node.server.connect(connect_addr);

    // Put a message on the SendMessageQueue of the sender node,
    // to be sent over UDP.
    // (NLL SVP.)
    {
        let send_queue = &mut sender_node.world.write_resource::<SendMessageQueue<TestMessage>>().queue;
        send_queue.push_back(
            SendMessage {
                // HACKS: There will only be exactly one peer,
                // so its ID will be 1. (Peer ID 0 is self.)
                // TODO: Give each peer an identity (GUID, then later a key pair),
                // and look it up by that.
                dest_peer_id: PeerId(1),
                game_message: TestMessage{
                    disposition: "Sunny!".to_string(),
                },
                transport: Transport::UDP,
            }
        );
    }

    // Step the world.
    // This should send the message.
    sender_node.dispatch();
    // Sleep a while to make sure we receive the message.
    std::thread::sleep(Duration::from_millis(100));
    // This should receive the message.
    receiver_node.dispatch();

    // (NLL SVP.)
    {
        // Take a look at what ended up in the received message queue.
        let recv_queue = &mut receiver_node.world.write_resource::<RecvMessageQueue<TestMessage>>().queue;
        // We should have received something equivalent to what we sent.
        assert_eq!(recv_queue.len(), 1);
        let expected_message = TestMessage {
            disposition: "Sunny!".to_string(),
        };
        assert_eq!(recv_queue.pop_front().unwrap().game_message, expected_message);
    }

    // Put two messages on the SendMessageQueue of the sender node,
    // to be sent over TCP.
    // (NLL SVP.)
    {
        let send_queue = &mut sender_node.world.write_resource::<SendMessageQueue<TestMessage>>().queue;
        send_queue.push_back(
            SendMessage {
                // HACKS: There will only be exactly one peer,
                // so its ID will be 1. (Peer ID 0 is self.)
                // TODO: Give each peer an identity (GUID, then later a key pair),
                // and look it up by that.
                dest_peer_id: PeerId(1),
                game_message: TestMessage{
                    disposition: "Cooperative!".to_string(),
                },
                transport: Transport::TCP,
            }
        );
        send_queue.push_back(
            SendMessage {
                dest_peer_id: PeerId(1),
                game_message: TestMessage{
                    disposition: "Enthusiastic!".to_string(),
                },
                transport: Transport::TCP,
            }
        );
    }

    // Step the world.
    // This should send the messages.
    sender_node.dispatch();
    // Sleep a while to make sure we receive the messages.
    std::thread::sleep(Duration::from_millis(100));
    // This should receive the messages.
    receiver_node.dispatch();

    // Take a look at what ended up in the received message queue.
    let recv_queue = &mut receiver_node.world.write_resource::<RecvMessageQueue<TestMessage>>().queue;
    // We should have received something equivalent to what we sent.
    assert_eq!(recv_queue.len(), 2);
    let expected_message1 = TestMessage {
        disposition: "Cooperative!".to_string(),
    };
    assert_eq!(recv_queue.pop_front().unwrap().game_message, expected_message1);
    let expected_message2 = TestMessage {
        disposition: "Enthusiastic!".to_string(),
    };
    assert_eq!(recv_queue.pop_front().unwrap().game_message, expected_message2);

    // TODO: gracefully shut down the server before the end of all tests;
    // you don't want to leave the thread hanging around awkwardly.
}
