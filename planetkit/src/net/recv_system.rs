use std::sync::mpsc;

use slog::Logger;
use specs;
use specs::{Read, Write};

use super::{
    GameMessage, NetworkPeers, RecvMessage, RecvMessageQueue, RecvWireMessage, WireMessage,
};

pub struct RecvSystem<G: GameMessage> {
    log: Logger,
    // Channel for slurping wire messages from network server.
    recv_rx: mpsc::Receiver<RecvWireMessage<G>>,
}

impl<G> RecvSystem<G>
where
    G: GameMessage,
{
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> RecvSystem<G> {
        // Take wire message receiver from ServerResource.
        use super::ServerResource;
        let server_resource = world.write_resource::<ServerResource<G>>();
        let recv_rx = server_resource
            .recv_rx
            .lock()
            .expect("Couldn't get lock on wire message receiver")
            .take()
            .expect("Somebody already took it!");

        RecvSystem {
            log: parent_log.new(o!()),
            recv_rx,
        }
    }
}

impl<'a, G> specs::System<'a> for RecvSystem<G>
where
    G: GameMessage,
{
    type SystemData = (Write<'a, RecvMessageQueue<G>>, Read<'a, NetworkPeers<G>>);

    fn run(&mut self, data: Self::SystemData) {
        let (mut recv_message_queue, network_peers) = data;

        // Slurp everything the server sent us.
        loop {
            let recv_wire_message = match self.recv_rx.try_recv() {
                Ok(recv_wire_message) => recv_wire_message,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => panic!("sender hung up"),
            };

            let src = recv_wire_message.src;
            let message = match recv_wire_message.message {
                Ok(message) => message,
                Err(_) => {
                    warn!(self.log, "Got garbled message"; "peer_addr" => format!("{:?}", src));
                    continue;
                }
            };

            // Figure out who sent it. Do this after decoding the body so we
            // can log some useful information about what was in the message.
            //
            // TODO: ruh roh, what if two clients connect from the same IP?
            // We need to make peers always identify themselves in every message,
            // (and then use the HMAC to validate identity and message).
            let peer_id = match network_peers
                .peers
                .iter()
                .find(|peer| peer.socket_addr == src)
            {
                Some(peer) => peer.id,
                None => {
                    warn!(self.log, "Got message from address we don't recognise; did they disconnect"; "peer_addr" => format!("{:?}", src), "message" => format!("{:?}", message));
                    continue;
                }
            };

            let game_message = match message {
                WireMessage::Game(game_message) => game_message,
                _ => {
                    warn!(
                        self.log,
                        "Don't yet know how to do anything with non-game messages"
                    );
                    continue;
                }
            };

            // TODO: Verify authenticity of message sender.
            // (All messages sent over the wire should include this,
            // initially as a plain assertion of their identity, and eventually
            // at least HMAC.)

            // Re-wrap the message for consumption by other systems.
            let recv_message = RecvMessage {
                source: peer_id,
                game_message,
            };
            recv_message_queue.queue.push_back(recv_message);
        }
    }
}
