use specs;
use specs::{Read, ReadExpect, WriteExpect, Entities, LazyUpdate, ReadStorage};
use slog::Logger;

use pk::Spatial;
use pk::net::{EntityIds, SendMessageQueue, Destination, Transport, SendMessage};
use pk::cell_dweller::CellDweller;

use ::message::Message;
use super::RecvMessageQueue;
use super::{WeaponMessage, NewGrenadeMessage};
use super::grenade::shoot_grenade;

pub struct RecvSystem {
    log: Logger,
}

impl RecvSystem {
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> RecvSystem {
        use pk::AutoResource;

        // Ensure resources we use are present.
        RecvMessageQueue::ensure(world);

        RecvSystem {
            log: parent_log.new(o!("system" => "weapon_recv"))
        }
    }
}

impl<'a> specs::System<'a> for RecvSystem {
    type SystemData = (
        WriteExpect<'a, RecvMessageQueue>,
        WriteExpect<'a, SendMessageQueue<Message>>,
        Entities<'a>,
        Read<'a, LazyUpdate>,
        ReadStorage<'a, Spatial>,
        ReadStorage<'a, CellDweller>,
        ReadExpect<'a, EntityIds>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut recv_message_queue,
            mut send_message_queue,
            entities,
            updater,
            spatials,
            cell_dwellers,
            entity_ids,
        ) = data;

        while let Some(message) = recv_message_queue.queue.pop_front() {
            match message.game_message {
                WeaponMessage::ShootGrenade(shoot_grenade_message) => {
                    // TODO: verify that we're the master

                    trace!(self.log, "Firing grenade because a peer asked me to"; "message" => format!("{:?}", shoot_grenade_message));

                    // NOTE: Hacks until we have saveload;
                    // just tell everyone including ourself to fire the grenade,
                    // and then only the server will actually trigger an explosion
                    // when the grenade runs out of time.
                    // TODO: not this!
                    send_message_queue.queue.push_back(
                        SendMessage {
                            destination: Destination::EveryoneIncludingSelf,
                            game_message: Message::Weapon(
                                WeaponMessage::NewGrenade(
                                    NewGrenadeMessage {
                                        fired_by_player_id: shoot_grenade_message.fired_by_player_id,
                                        fired_by_cell_dweller_entity_id: shoot_grenade_message.fired_by_cell_dweller_entity_id,
                                    }
                                )
                            ),
                            // TODO: does it matter if we miss one — maybe UDP?
                            // TCP for now, then solve this by having TTL on some entities.
                            // Or a standard "TTL / clean-me-up" component type! :)
                            transport: Transport::TCP,
                        }
                    );
                },
                WeaponMessage::NewGrenade(new_grenade_message) => {
                    trace!(self.log, "Spawning grenade because server asked me to"; "message" => format!("{:?}", new_grenade_message));

                    // Look up the entity from its global ID.
                    let cell_dweller_entity = match entity_ids.mapping.get(&new_grenade_message.fired_by_cell_dweller_entity_id) {
                        Some(ent) => ent.clone(),
                        // We probably just don't know about it yet.
                        None => {
                            debug!(self.log, "Unknown CellDweller fired a grenade"; "entity_id" => new_grenade_message.fired_by_cell_dweller_entity_id);
                            continue;
                        },
                    };

                    shoot_grenade(
                        &entities,
                        &updater,
                        &cell_dwellers,
                        cell_dweller_entity,
                        &spatials,
                        &self.log,
                        new_grenade_message.fired_by_player_id,
                    );
                },
            }
        }
    }
}
