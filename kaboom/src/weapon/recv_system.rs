use specs;
use specs::{Fetch, FetchMut, Entities, LazyUpdate, ReadStorage};
use slog::Logger;

use pk::Spatial;
use pk::net::{EntityIds};
use pk::cell_dweller::CellDweller;

use super::RecvMessageQueue;
use super::WeaponMessage;
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
        FetchMut<'a, RecvMessageQueue>,
        Entities<'a>,
        Fetch<'a, LazyUpdate>,
        ReadStorage<'a, Spatial>,
        ReadStorage<'a, CellDweller>,
        Fetch<'a, EntityIds>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut recv_message_queue,
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

                    // TODO: demote to trace
                    info!(self.log, "Firing grenade because a peer asked me to"; "message" => format!("{:?}", shoot_grenade_message));

                    // Look up the entity from its global ID.
                    let cell_dweller_entity = match entity_ids.mapping.get(&shoot_grenade_message.fired_by_cell_dweller_entity_id) {
                        Some(ent) => ent.clone(),
                        // We probably just don't know about it yet.
                        None => {
                            debug!(self.log, "Unknown CellDweller fired a grenade"; "entity_id" => shoot_grenade_message.fired_by_cell_dweller_entity_id);
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
                        shoot_grenade_message.fired_by_player_id,
                    );

                    // TODO: tell everyone else that it exists.
                },
            }
        }
    }
}
