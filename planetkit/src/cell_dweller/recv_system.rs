use slog::Logger;
use specs;
use specs::{Read, Write, WriteStorage};

use super::{
    CellDweller, CellDwellerMessage, RecvMessageQueue, RemoveBlockMessage, SendMessage,
    SendMessageQueue,
};
use globe::Globe;
use grid::PosInOwningRoot;
use net::{Destination, EntityIds, NodeResource, Transport};
use Spatial;

pub struct RecvSystem {
    log: Logger,
}

impl RecvSystem {
    pub fn new(parent_log: &Logger) -> RecvSystem {
        RecvSystem {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for RecvSystem {
    type SystemData = (
        WriteStorage<'a, Globe>,
        WriteStorage<'a, CellDweller>,
        WriteStorage<'a, Spatial>,
        Write<'a, RecvMessageQueue>,
        Write<'a, SendMessageQueue>,
        Read<'a, EntityIds>,
        Read<'a, NodeResource>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            mut globes,
            mut cell_dwellers,
            mut spatials,
            mut recv_message_queue,
            mut send_message_queue,
            entity_ids,
            node_resource,
        ) = data;

        // Slurp all inbound messages.
        while let Some(message) = recv_message_queue.queue.pop_front() {
            match message.game_message {
                CellDwellerMessage::SetPos(set_pos_message) => {
                    // Look up the entity from its global ID.
                    let cell_dweller_entity = match entity_ids
                        .mapping
                        .get(&set_pos_message.entity_id)
                    {
                        Some(ent) => ent,
                        // We probably just don't know about it yet.
                        None => {
                            // TODO: demote to trace
                            info!(self.log, "Heard about cell dweller we don't know about yet"; "entity_id" => set_pos_message.entity_id);
                            continue;
                        }
                    };
                    let cd = cell_dwellers
                        .get_mut(*cell_dweller_entity)
                        .expect("Missing CellDweller");
                    let spatial = spatials
                        .get_mut(*cell_dweller_entity)
                        .expect("Missing Spatial");

                    // TODO: validate that they're allowed to move this cell dweller.

                    // TODO: demote to trace
                    debug!(self.log, "Moving cell dweller because of received network message"; "message" => format!("{:?}", set_pos_message));

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

                    // Inform all peers that don't yet know about this action.
                    // TODO: do we need some kind of pattern for an action,
                    // where it's got rules for:
                    // - how to turn it into a request if we're not the server.
                    // - how to action it if we are the server.
                    // - how to action it if we are a client. (Maybe it's just
                    //   a different kind of message in that case.)
                    // - how to forward it on if we're the server and we just
                    //   acted on it.
                    if node_resource.is_master {
                        send_message_queue.queue.push_back(SendMessage {
                            destination: Destination::EveryoneElseExcept(message.source),
                            game_message: CellDwellerMessage::SetPos(set_pos_message),
                            transport: Transport::UDP,
                        })
                    }
                }
                CellDwellerMessage::TryPickUpBlock(try_pick_up_block_message) => {
                    // TODO: validate that we are the server.

                    // Look up the entity from its global ID.
                    let cell_dweller_entity = match entity_ids
                        .mapping
                        .get(&try_pick_up_block_message.cd_entity_id)
                    {
                        Some(ent) => ent,
                        // We probably just don't know about it yet.
                        None => {
                            // TODO: demote to trace
                            info!(self.log, "Heard about cell dweller we don't know about yet"; "entity_id" => try_pick_up_block_message.cd_entity_id);
                            continue;
                        }
                    };
                    let cd = cell_dwellers
                        .get_mut(*cell_dweller_entity)
                        .expect("Missing CellDweller");

                    // Get the associated globe, complaining loudly if we fail.
                    // TODO: again, need a pattern for this that isn't awful.
                    let globe_entity = match cd.globe_entity {
                        Some(globe_entity) => globe_entity,
                        None => {
                            warn!(
                                self.log,
                                "There was no associated globe entity or it wasn't actually a Globe! Can't proceed!"
                            );
                            continue;
                        }
                    };
                    let globe = match globes.get_mut(globe_entity) {
                        Some(globe) => globe,
                        None => {
                            warn!(
                                self.log,
                                "The globe associated with this CellDweller is not alive! Can't proceed!"
                            );
                            continue;
                        }
                    };

                    // TODO: validate that peer is allowed to remove the block.
                    // TODO: handle their source position and target pickup spot.
                    // Initially just trust the client is honest.
                    let maybe_cell_info = super::mining::pick_up_if_possible(cd, globe);
                    if let Some((new_pos_in_owning_root, cell)) = maybe_cell_info {
                        debug!(self.log, "Removed a block because a peer asked"; "pos" => format!("{:?}", new_pos_in_owning_root), "cell" => format!("{:?}", cell));

                        // Tell everyone else what happened.
                        //
                        // TODO: this needs to be a much more sophisticated kind of messaging
                        // about chunk content changing over time, not sending it to clients
                        // that clearly shouldn't need to care, allowing clients to
                        // ignore messages they receive if they decide they don't care,
                        // then catch up later, etc.
                        //
                        // But for now, just say "this block is gone"!
                        let remove_block_message = RemoveBlockMessage {
                            // TODO: identify the globe. But for that we'd first need
                            // the server to inform clients about the globe it created.
                            // For now we'll just use the first globe we find.
                            // globe_entity_id: ......,
                            pos: new_pos_in_owning_root.into(),
                        };
                        send_message_queue.queue.push_back(SendMessage {
                            destination: Destination::EveryoneElse,
                            game_message: CellDwellerMessage::RemoveBlock(remove_block_message),
                            transport: Transport::TCP,
                        });
                    }
                }
                CellDwellerMessage::RemoveBlock(remove_block_message) => {
                    // For now just find the first globe, and assume that's
                    // the one we're supposed to be working with.
                    use specs::Join;
                    let globe = (&mut globes)
                        .join()
                        .next()
                        .expect("Should've been at least one globe.");

                    // TODO: validate that position makes sense. Don't want the client
                    // to be able to punk us.

                    let pos_in_owning_root = PosInOwningRoot::new(
                        remove_block_message.pos,
                        globe.spec().root_resolution,
                    );
                    let removed_cell = super::mining::remove_block(globe, pos_in_owning_root);

                    debug!(self.log, "Removed a block master told me to"; "pos" => format!("{:?}", remove_block_message.pos), "cell" => format!("{:?}", removed_cell));
                }
            }
        }
    }
}
