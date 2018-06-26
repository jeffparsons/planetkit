use specs;
use specs::{Write};
use slog::Logger;

use pk::cell_dweller;
use pk::net::{
    RecvMessage,
    RecvMessageQueue,
};

use ::message::Message;
use ::player;
use ::weapon;

pub struct RecvDemuxSystem{
    log: Logger,
}

impl RecvDemuxSystem {
    pub fn new(parent_log: &Logger, _world: &mut specs::World) -> RecvDemuxSystem {
        RecvDemuxSystem {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for RecvDemuxSystem {
    type SystemData = (
        Write<'a, RecvMessageQueue<Message>>,
        Write<'a, cell_dweller::RecvMessageQueue>,
        Write<'a, player::RecvMessageQueue>,
        Write<'a, weapon::RecvMessageQueue>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut recv_message_queue,
            mut cell_dweller_recv_queue,
            mut player_recv_queue,
            mut weapon_recv_queue,
        ) = data;

        // Drain the recv message queue, and dispatch to system-specific queues.
        while let Some(message) = recv_message_queue.queue.pop_front() {
            match message.game_message {
                Message::CellDweller(cd_message) => {
                    trace!(self.log, "Forwarding cell dweller message to its recv message queue"; "message" => format!("{:?}", cd_message));
                    cell_dweller_recv_queue.queue.push_back(
                        RecvMessage {
                            source: message.source,
                            game_message: cd_message,
                        }
                    );
                },
                Message::Player(player_message) => {
                    trace!(self.log, "Forwarding player message to its recv message queue"; "message" => format!("{:?}", player_message));
                    player_recv_queue.queue.push_back(
                        RecvMessage {
                            source: message.source,
                            game_message: player_message,
                        }
                    );
                },
                Message::Weapon(weapon_message) => {
                    trace!(self.log, "Forwarding weapon message to its recv message queue"; "message" => format!("{:?}", weapon_message));
                    weapon_recv_queue.queue.push_back(
                        RecvMessage {
                            source: message.source,
                            game_message: weapon_message,
                        }
                    );
                },
            }
        }
    }
}
