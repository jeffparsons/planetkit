use specs;
use specs::{WriteStorage, Fetch, FetchMut};
use slog::Logger;

use super::{
    CellDweller,
    ActiveCellDweller,
    RecvMessageQueue,
    CellDwellerMessage,
};
use Spatial;

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
        Fetch<'a, ActiveCellDweller>,
        FetchMut<'a, RecvMessageQueue>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut cell_dwellers,
            mut spatials,
            active_cell_dweller_resource,
            mut recv_message_queue
        ) = data;

        // Slurp all inbound messages.
        while let Some(message) = recv_message_queue.queue.pop_front() {
            match message.game_message {
                CellDwellerMessage::SetPos(set_pos_message) => {
                    let active_cell_dweller_entity = match active_cell_dweller_resource.maybe_entity {
                        Some(entity) => entity,
                        None => return,
                    };
                    let cd = cell_dwellers.get_mut(active_cell_dweller_entity).expect(
                        "Someone deleted the controlled entity's CellDweller",
                    );
                    let spatial = spatials.get_mut(active_cell_dweller_entity).expect(
                        "Someone deleted the controlled entity's Spatial",
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
