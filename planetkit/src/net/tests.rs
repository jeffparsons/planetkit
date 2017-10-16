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
    pub fn new() -> Node {
        let drain = slog::Discard;
        let root_log = slog::Logger::root(drain, o!("pk_version" => env!("CARGO_PKG_VERSION")));

        // Create sender and receiver systems.
        let mut world = specs::World::new();
        let recv_system = RecvSystem::<TestMessage>::new(&root_log, &mut world);
        let mut send_system = SendSystem::<TestMessage>::new(&root_log, &mut world);

        // Spawn TCP and UDP server/client.
        let server = Server::new(
            &root_log,
            recv_system.sender().clone(),
            send_system.take_new_peer_sender().expect("Somebody else took it!"),
            send_system.take_send_udp_wire_message_rx().expect("Somebody else took it!"),
        );

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

    pub fn new_server() -> Node {
        let mut server_node = Node::new();
        server_node.server.start_listen(None);
        server_node
    }

    pub fn new_client_connected_to(server_node: &Node) -> Node {
        let mut client_node = Node::new();
        let connect_addr = format!("127.0.0.1:{}", server_node.server.port.expect("Should be listening"));
        let connect_addr: SocketAddr = connect_addr.parse().unwrap();
        client_node.server.connect(connect_addr);
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

    // Put a message on the SendMessageQueue of the client node,
    // to be sent over UDP.
    client_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            dest_peer_id: PeerId(1),
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

    // Testing multiple TCP messages is kind of interesting
    // because we need to make sure we don't corrupt the
    // stream/buffer when receiving them, as opposed to UDP
    // where we work with individual datagrams.
    client_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            dest_peer_id: PeerId(1),
            game_message: TestMessage{
                disposition: "Cooperative!".to_string(),
            },
            transport: Transport::TCP,
        }
    );
    client_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            dest_peer_id: PeerId(1),
            game_message: TestMessage{
                disposition: "Enthusiastic!".to_string(),
            },
            transport: Transport::TCP,
        }
    );

    // Step the world.
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

    // Put a message on the SendMessageQueue of the server node,
    // to be sent over UDP.
    server_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            dest_peer_id: PeerId(1),
            game_message: TestMessage{
                disposition: "Authoritative!".to_string(),
            },
            transport: Transport::UDP,
        }
    );

    // Sleep a while to make sure the server has
    // registered the new client as a peer before
    // trying to send to it.
    std::thread::sleep(Duration::from_millis(10));

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

    // Testing multiple TCP messages is kind of interesting
    // because we need to make sure we don't corrupt the
    // stream/buffer when receiving them, as opposed to UDP
    // where we work with individual datagrams.
    server_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            dest_peer_id: PeerId(1),
            game_message: TestMessage{
                disposition: "Oppressive!".to_string(),
            },
            transport: Transport::TCP,
        }
    );
    server_node.enqueue_message(
        SendMessage {
            // Peer ID 0 is self.
            dest_peer_id: PeerId(1),
            game_message: TestMessage{
                disposition: "Demanding!".to_string(),
            },
            transport: Transport::TCP,
        }
    );

    // Sleep a while to make sure the server has
    // registered the new client as a peer before
    // trying to send to it.
    std::thread::sleep(Duration::from_millis(10));

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
