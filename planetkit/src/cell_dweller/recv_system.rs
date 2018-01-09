use specs;
use specs::{WriteStorage, Fetch, FetchMut};
use slog::Logger;

use super::{
    CellDweller,
    RecvMessageQueue,
    CellDwellerMessage,
};
use Spatial;

use net::EntityIds;

pub struct RecvSystem {
    log: Logger,
}

impl RecvSystem {
    pub fn new(
        world: &mut specs::World,
        parent_log: &Logger,
    ) -> RecvSystem {
        use ::AutoResource;
        RecvMessageQueue::ensure(world);

        RecvSystem {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for RecvSystem {
    type SystemData = (
        WriteStorage<'a, CellDweller>,
        WriteStorage<'a, Spatial>,
        FetchMut<'a, RecvMessageQueue>,
        Fetch<'a, EntityIds>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut cell_dwellers,
            mut spatials,
            mut recv_message_queue,
            entity_ids,
        ) = data;

        // Slurp all inbound messages.
        while let Some(message) = recv_message_queue.queue.pop_front() {
            match message.game_message {
                CellDwellerMessage::SetPos(set_pos_message) => {
                    // Look up the entity from its global ID.
                    let cell_dweller_entity = match entity_ids.mapping.get(&set_pos_message.entity_id) {
                        Some(ent) => ent,
                        // We probably just don't know about it yet.
                        None => {
                            // TODO: demote to trace
                            info!(self.log, "Heard about cell dweller we don't know about yet"; "entity_id" => set_pos_message.entity_id);
                            continue;
                        },
                    };
                    let cd = cell_dwellers.get_mut(*cell_dweller_entity).expect(
                        "Missing CellDweller",
                    );
                    let spatial = spatials.get_mut(*cell_dweller_entity).expect(
                        "Missing Spatial",
                    );

                    // TODO: demote to trace
                    info!(self.log, "Moving cell dweller because of received network message"; "message" => format!("{:?}", set_pos_message));

                    // TODO: move a specific cell dweller,
                    // not whichever one is active. :)

                    cd.set_cell_transform(
                        set_pos_message.new_pos,
                        set_pos_message.new_dir,
                        set_pos_message.new_last_turn_bias,
                    );

                    // TODO: tell all _other_ peers about this update.
                    // IFF we are the server.
                    // TODO: do we need some kind of pattern for an action,
                    // where it's got rules for:
                    // - how to turn it into a request if we're not the server.
                    // - how to action it if we are the server.
                    // - how to action it if we are a client. (Maybe it's just
                    //   a different kind of message in that case.)
                    // - how to forward it on if we're the server and we just
                    //   acted on it.

                    // Update real-space coordinates if necessary.
                    // TODO: do this in a separate system; it needs to be done before
                    // things are rendered, but there might be other effects like gravity,
                    // enemies shunting the cell dweller around, etc. that happen
                    // after control.
                    if cd.is_real_space_transform_dirty() {
                        spatial.set_local_transform(cd.get_real_transform_and_mark_as_clean());
                    }
                },
            }
        }
    }
}
