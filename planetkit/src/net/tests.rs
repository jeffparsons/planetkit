use std;
use std::time::Duration;

use slog;
use specs;

use super::*;

// Nothing interesting in here!
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
struct TestMessage {
    disposition: String,
}
impl GameMessage for TestMessage {}

// Network node helper. Contains all the network
// server bits and Specs bits required to simulate
// a network node, so we can easily play with multiple.
struct Node {
    world: specs::World,
    dispatcher: specs::Dispatcher<'static, 'static>,
}

impl Node {
    pub fn new() -> Node {
        use ::LogResource;

        let drain = slog::Discard;
        let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));

        let mut world = specs::World::new();

        // Initialize common resources.
        // These should be impossible to create from
        // just a `World`; `pk::Resource` should be
        // preferred to ensure those.
        world.add_resource(LogResource::new(&root_log));

        // Create core network systems.
        let new_peer_system = NewPeerSystem::<TestMessage>::new(&root_log, &mut world);
        let recv_system = RecvSystem::<TestMessage>::new(&root_log, &mut world);
        let send_system = SendSystem::<TestMessage>::new(&root_log, &mut world);

        // Make a dispatcher, with simplified execution order.
        // TODO: replace this with waiting for specific things to be ready,
        // so you don't need to fiddle with fine timings.
        let dispatcher = specs::DispatcherBuilder::new()
            .with(new_peer_system, "new_peer", &[])
            .with_barrier()
            .with(recv_system, "recv", &[])
            .with_barrier()
            .with(send_system, "send", &[])
            .build();

        // There's no system ensuring these exist in tests,
        // so we'll need to do it ourselves.
        world.setup::<specs::Write<SendMessageQueue<TestMessage>>>();
        world.setup::<specs::Write<RecvMessageQueue<TestMessage>>>();
        world.setup::<specs::Write<NodeResource>>();

        Node {
            world: world,
            dispatcher: dispatcher,
        }
    }

    pub fn new_server() -> Node {
        let server_node = Node::new();
        {
            // NLL SVP
            let server_resource = server_node.world.read_resource::<ServerResource<TestMessage>>();
            let mut server = server_resource.server.lock().expect("Couldn't lock server");
            server.start_listen(None);
        }
        server_node
    }

    pub fn new_client_connected_to(server_node: &Node) -> Node {
        let client_node = Node::new();
        let server_server_resource = server_node.world.read_resource::<ServerResource<TestMessage>>();
        let server_server = server_server_resource.server.lock().expect("Couldn't lock server");
        let connect_addr = format!("127.0.0.1:{}", server_server.port.expect("Should be listening"));
        let connect_addr: SocketAddr = connect_addr.parse().unwrap();
        {
            // NLL SVP
            let client_server_resource = client_node.world.read_resource::<ServerResource<TestMessage>>();
            let mut client_server = client_server_resource.server.lock().expect("Couldn't lock server");
            client_server.connect(connect_addr);
        }
        client_node
    }

    pub fn dispatch(&mut self) {
        self.dispatcher.dispatch(&mut self.world.res);
    }

    pub fn enqueue_message(&mut self, message: SendMessage<TestMessage>) {
        let send_queue = &mut self.world.write_resource::<SendMessageQueue<TestMessage>>().queue;
        send_queue.push_back(message);
    }

    pub fn expect_message(&mut self, expected_message: TestMessage) {
        let recv_queue = &mut self.world.write_resource::<RecvMessageQueue<TestMessage>>().queue;
        assert!(recv_queue.len() >= 1);
        let received_message = recv_queue.pop_front().unwrap().game_message;
        assert_eq!(received_message, expected_message);
    }
}

#[test]
fn client_sends_udp_message_to_server() {
    let mut server_node = Node::new_server();
    let mut client_node = Node::new_client_connected_to(&server_node);

    // Give the server a chance to register the new peer,
    // so that when first check for a message, there's one
    // already received from the TCP stack.
    //
    // Client node should do this shortly before it sends its message,
    // because it gets a chance to dispatch anyway.
    //
    // These are pretty low-level tests, so this is ok here,
    // but higher-level tests should all have sensible timeouts
    // etc. instead.
    std::thread::sleep(Duration::from_millis(10));
    server_node.dispatch();

    // Put a message on the SendMessageQueue of the client node,
    // to be sent over UDP.
    client_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            destination: Destination::One(PeerId(1)),
            game_message: TestMessage{
                disposition: "Sunny!".to_string(),
            },
            transport: Transport::UDP,
        }
    );

    // Step the world.
    // This should send the message.
    client_node.dispatch();
    // Sleep a while to make sure we receive the message.
    std::thread::sleep(Duration::from_millis(10));
    // This should receive the message.
    server_node.dispatch();

    // Server should have received equivalent message.
    server_node.expect_message(TestMessage {
        disposition: "Sunny!".to_string(),
    });

    // TODO: gracefully shut down the server before the end of all tests;
    // you don't want to leave the thread hanging around awkwardly.
}

#[test]
fn client_sends_tcp_messages_to_server() {
    let mut server_node = Node::new_server();
    let mut client_node = Node::new_client_connected_to(&server_node);

    // Give the server a chance to register the new peer,
    // so that when first check for a message, there's one
    // already received from the TCP stack.
    //
    // Client node should do this shortly before it sends its message,
    // because it gets a chance to dispatch anyway.
    //
    // These are pretty low-level tests, so this is ok here,
    // but higher-level tests should all have sensible timeouts
    // etc. instead.
    std::thread::sleep(Duration::from_millis(10));
    server_node.dispatch();

    // Testing multiple TCP messages is kind of interesting
    // because we need to make sure we don't corrupt the
    // stream/buffer when receiving them, as opposed to UDP
    // where we work with individual datagrams.
    client_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            destination: Destination::One(PeerId(1)),
            game_message: TestMessage{
                disposition: "Cooperative!".to_string(),
            },
            transport: Transport::TCP,
        }
    );
    client_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            destination: Destination::One(PeerId(1)),
            game_message: TestMessage{
                disposition: "Enthusiastic!".to_string(),
            },
            transport: Transport::TCP,
        }
    );

    // This should send the message.
    client_node.dispatch();
    // Sleep a while to make sure we receive the message.
    std::thread::sleep(Duration::from_millis(10));
    // This should receive the message.
    server_node.dispatch();

    // Server should have received equivalent messages, in order.
    server_node.expect_message(TestMessage {
        disposition: "Cooperative!".to_string(),
    });
    server_node.expect_message(TestMessage {
        disposition: "Enthusiastic!".to_string(),
    });

    // TODO: gracefully shut down the server before the end of all tests;
    // you don't want to leave the thread hanging around awkwardly.
}

#[test]
fn server_sends_udp_message_to_client() {
    let mut server_node = Node::new_server();
    let mut client_node = Node::new_client_connected_to(&server_node);

    // Give the client a chance to register the new peer,
    // so that when first check for a message, there's one
    // already received from the UDP stack.
    //
    // Server node should do this shortly before it sends its message,
    // because it gets a chance to dispatch anyway.
    //
    // These are pretty low-level tests, so this is ok here,
    // but higher-level tests should all have sensible timeouts
    // etc. instead.
    std::thread::sleep(Duration::from_millis(10));
    client_node.dispatch();

    // Put a message on the SendMessageQueue of the server node,
    // to be sent over UDP.
    server_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            destination: Destination::One(PeerId(1)),
            game_message: TestMessage{
                disposition: "Authoritative!".to_string(),
            },
            transport: Transport::UDP,
        }
    );

    // Step the world.
    // This should send the message.
    server_node.dispatch();
    // Sleep a while to make sure we receive the message.
    std::thread::sleep(Duration::from_millis(10));
    // This should receive the message.
    client_node.dispatch();

    // Client should have received equivalent message.
    client_node.expect_message(TestMessage {
        disposition: "Authoritative!".to_string(),
    });

    // TODO: gracefully shut down the server before the end of all tests;
    // you don't want to leave the thread hanging around awkwardly.
}

#[test]
fn server_sends_tcp_messages_to_client() {
    let mut server_node = Node::new_server();
    let mut client_node = Node::new_client_connected_to(&server_node);

    // Give the client a chance to register the new peer,
    // so that when first check for a message, there's one
    // already received from the UDP stack.
    //
    // Server node should do this shortly before it sends its message,
    // because it gets a chance to dispatch anyway.
    //
    // These are pretty low-level tests, so this is ok here,
    // but higher-level tests should all have sensible timeouts
    // etc. instead.
    std::thread::sleep(Duration::from_millis(10));
    client_node.dispatch();

    // Testing multiple TCP messages is kind of interesting
    // because we need to make sure we don't corrupt the
    // stream/buffer when receiving them, as opposed to UDP
    // where we work with individual datagrams.
    server_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            destination: Destination::One(PeerId(1)),
            game_message: TestMessage{
                disposition: "Oppressive!".to_string(),
            },
            transport: Transport::TCP,
        }
    );
    server_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            destination: Destination::One(PeerId(1)),
            game_message: TestMessage{
                disposition: "Demanding!".to_string(),
            },
            transport: Transport::TCP,
        }
    );

    // Step the world.
    // This should send the message.
    server_node.dispatch();
    // Sleep a while to make sure we receive the message.
    std::thread::sleep(Duration::from_millis(10));
    // This should receive the message.
    client_node.dispatch();

    // Client should have received equivalent messages, in order.
    client_node.expect_message(TestMessage {
        disposition: "Oppressive!".to_string(),
    });
    client_node.expect_message(TestMessage {
        disposition: "Demanding!".to_string(),
    });

    // TODO: gracefully shut down the server before the end of all tests;
    // you don't want to leave the thread hanging around awkwardly.
}
