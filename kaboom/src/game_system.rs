use specs;
use specs::{ReadStorage, WriteStorage, Fetch, FetchMut, LazyUpdate, Entities};
use slog::Logger;

use pk;
use pk::globe::Globe;
use pk::cell_dweller::{CellDweller, ActiveCellDweller};
use pk::camera::DefaultCamera;
use pk::net::{NodeResource, PeerId, NetworkPeers, Destination, Transport, SendMessageQueue, SendMessage, EntityIds, NetMarker};

use ::player::{self, Player, PlayerId, PlayerMessage};
use ::game_state::GameState;
use ::client_state::ClientState;
use ::planet;
use ::fighter;
use ::message::Message;

/// System to drive the top-level state machine for level and game state.
pub struct GameSystem {
    log: Logger,
}

// TODO: split most of this out into a "new player" system.

impl GameSystem {
    pub fn new(parent_log: &Logger, world: &mut specs::World) -> GameSystem {
        use pk::AutoResource;

        // Ensure resources we use are present.
        NodeResource::ensure(world);
        GameState::ensure(world);
        ClientState::ensure(world);
        player::RecvMessageQueue::ensure(world);
        EntityIds::ensure(world);

        GameSystem {
            log: parent_log.new(o!("system" => "game"))
        }
    }

    fn create_and_broadcast_player(
        &mut self,
        game_state: &mut FetchMut<GameState>,
        send_message_queue: &mut FetchMut<SendMessageQueue<Message>>,
        peer_id: PeerId,
    ) {
        let next_player_id = PlayerId(game_state.players.len() as u16);
        game_state.players.push(
            Player {
                id: next_player_id,
                peer_id: peer_id,
                fighter_entity: None,
            }
        );
        game_state.new_players.push_back(next_player_id);

        // Tell all the other peers about this new player.
        send_message_queue.queue.push_back(
            SendMessage {
                destination: Destination::EveryoneElse,
                game_message: Message::Player(
                    PlayerMessage::NewPlayer(next_player_id)
                ),
                transport: Transport::TCP,
            }
        );

        // Tell the owner (even if it's us) who their new player is.
        send_message_queue.queue.push_back(
            SendMessage {
                destination: Destination::One(peer_id),
                game_message: Message::Player(
                    PlayerMessage::YourPlayer(next_player_id)
                ),
                transport: Transport::TCP,
            }
        );
    }
}

impl<'a> specs::System<'a> for GameSystem {
    type SystemData = (
        Fetch<'a, NodeResource>,
        FetchMut<'a, GameState>,
        FetchMut<'a, ClientState>,
        Entities<'a>,
        Fetch<'a, LazyUpdate>,
        WriteStorage<'a, Globe>,
        FetchMut<'a, ActiveCellDweller>,
        WriteStorage<'a, CellDweller>,
        FetchMut<'a, DefaultCamera>,
        FetchMut<'a, NetworkPeers<Message>>,
        FetchMut<'a, SendMessageQueue<Message>>,
        FetchMut<'a, player::RecvMessageQueue>,
        FetchMut<'a, EntityIds>,
        ReadStorage<'a, NetMarker>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            node_resource,
            mut game_state,
            mut client_state,
            entities,
            updater,
            mut globes,
            mut active_cell_dweller,
            cell_dwellers,
            mut default_camera,
            mut network_peers,
            mut send_message_queue,
            mut player_recv_message_queue,
            mut entity_ids,
            net_markers,
        ) = data;

        // TODO: eventually only the server should create this, and then describe it to clients.
        // But for now, we need to make sure we create it _and_ it is realised before we try to
        // process any network messages.
        if game_state.globe_entity.is_none() {
            // Create the globe first, because we'll need it to figure out where
            // to place the player character.
            game_state.globe_entity = Some(
                planet::create(&entities, &updater)
            );

            // Don't do anything else in the GameSystem for the rest of the frame.
            // All we're really trying to achieve here is to not process any messages
            // about components that haven't been realised, but this is a temporary
            // solution and it doesn't hurt to skip a frame. (...until I accidentally
            // put something really important down below and it ruins everything!)
            return;
        }

        while let Some(message) = player_recv_message_queue.queue.pop_front() {
            match message.game_message {
                PlayerMessage::NewPlayer(player_id) => {
                    // Add the new player to our list.

                    // Master should never be told about new players.
                    // TODO: don't explode, just log a loud error,
                    // and kick the client who sent the bad message.
                    // You need a pattern for this.
                    debug_assert!(!node_resource.is_master);

                    if (player_id.0 as usize) < game_state.players.len() {
                        // TODO: demote to debug
                        info!(self.log, "Heard about new player we already know about"; "player_id" => format!("{:?}", player_id));
                        continue;
                    }

                    // We shouldn't hear about new players until
                    // we've at least caught up on existing ones.
                    assert!((player_id.0 as usize) <= game_state.players.len());
                    game_state.players.push(
                        Player {
                            id: player_id,
                            // TODO: don't just make this up!
                            // TODO: make the network server tack on
                            // the ID of the peer that sent these messages!!!!!
                            peer_id: PeerId(1),
                            fighter_entity: None,
                        }
                    );
                },
                PlayerMessage::YourPlayer(player_id) => {
                    // We should already know about this player.
                    assert!((player_id.0 as usize) < game_state.players.len());

                    info!(self.log, "Heard about our player"; "player_id" => format!("{:?}", player_id));

                    // Remember which player is ours.
                    client_state.player_id = Some(player_id);
                },
                PlayerMessage::NewFighter(entity_id) => {
                    debug!(self.log, "Heard about new fighter entity"; "entity_id" => entity_id);

                    // TODO: make sure we initialize everything in the right order,
                    // and have some way to queue things up until the entities they
                    // depend on are realized.
                    let globe_entity = game_state.globe_entity.expect("Should've had a globe entity by now.");
                    let mut globe = globes.get_mut(globe_entity).expect("Should've had a globe by now!");

                    // Create the player character.
                    let fighter_entity = fighter::create(
                        &entities,
                        &updater,
                        globe_entity,
                        &mut globe,
                    );
                    updater.insert(
                        fighter_entity,
                        NetMarker{ id: entity_id },
                    );

                    // Record its global ID so we can tell other peers
                    // about what we want to do to it.
                    entity_ids.mapping.insert(entity_id, fighter_entity);
                },
                PlayerMessage::YourFighter(entity_id) => {
                    debug!(self.log, "Heard about my fighter entity"; "entity_id" => entity_id);

                    let player_id = client_state.player_id.expect("Should've had a player ID by now.");
                    let player = &mut game_state.players[player_id.0 as usize];
                    let fighter_entity = entity_ids.mapping[&entity_id];
                    player.fighter_entity = Some(fighter_entity);
                },
            }
        }

        // If we are the master, but we don't yet know what our player is,
        // then insert a new player for us now. We'll hear about it on the
        // next tick, and register it as our own.
        if node_resource.is_master && client_state.player_id.is_none() {
            self.create_and_broadcast_player(&mut game_state, &mut send_message_queue, PeerId(0));
        }

        // If there are any new network peers, then pop them off
        // and maybe do something with them.
        while let Some(new_peer_id) = network_peers.new_peers.pop_front() {
            // As a client, we don't care; we just want to clean out the list.
            if node_resource.is_master {
                // Tell the new peer about all existing players.
                for player in &game_state.players {
                    send_message_queue.queue.push_back(
                        SendMessage {
                            destination: Destination::One(new_peer_id),
                            game_message: Message::Player(
                                PlayerMessage::NewPlayer(player.id)
                            ),
                            transport: Transport::TCP,
                        }
                    );
                }

                // Tell the new peer about all existing fighters.
                use specs::Join;
                for (_cd, net_marker) in (&cell_dwellers, &net_markers).join() {
                    send_message_queue.queue.push_back(
                        SendMessage {
                            destination: Destination::One(new_peer_id),
                            game_message: Message::Player(
                                PlayerMessage::NewFighter(net_marker.id)
                            ),
                            transport: Transport::TCP,
                        }
                    );
                }

                // Create a new player for that peer.
                self.create_and_broadcast_player(&mut game_state, &mut send_message_queue, new_peer_id);
            }

            // TODO: instead first just create a player for them,
            // then tell them about all existing players,
            // and then which player is theirs.
        }

        // Create a new character for each new player.
        if node_resource.is_master {
            if let Some(globe_entity) = game_state.globe_entity {
                // We can only do this after the globe has been realized.
                if let Some(mut globe) = globes.get_mut(globe_entity) {
                    while let Some(player_id) = game_state.new_players.pop_front() {
                        info!(self.log, "Found a new player; making a fighter for them"; "player_id" => format!("{:?}", player_id));

                        // Create the player character.
                        let fighter_entity = fighter::create(
                            &entities,
                            &updater,
                            globe_entity,
                            &mut globe,
                        );

                        let player = &mut game_state.players[player_id.0 as usize];
                        player.fighter_entity = Some(fighter_entity);

                        // Allocate a global ID so we can tell network
                        // peers about it.
                        let entity_id = entity_ids.range.next().expect("We ran out of IDs!");
                        entity_ids.mapping.insert(entity_id, fighter_entity);
                        updater.insert(
                            fighter_entity,
                            NetMarker{ id: entity_id },
                        );

                        // Tell all network peers about the new entity.
                        // TODO: use Specs's `saveload` stuff once it's in a release.
                        send_message_queue.queue.push_back(
                            SendMessage {
                                destination: Destination::EveryoneElse,
                                game_message: Message::Player(
                                    PlayerMessage::NewFighter(entity_id)
                                ),
                                transport: Transport::TCP,
                            }
                        );

                        // Tell the owner that this is _their_ fighter.
                        // TODO: this should probably tell them what
                        // player it's for, too. Splitscreen, bots, etc.
                        let peer_id = player.peer_id;
                        send_message_queue.queue.push_back(
                            SendMessage {
                                destination: Destination::One(peer_id),
                                game_message: Message::Player(
                                    PlayerMessage::YourFighter(entity_id)
                                ),
                                transport: Transport::TCP,
                            }
                        );
                    }
                }
            }
        }

        // TODO: index in a HashMap or something by PlayerId?
        // At the moment you can't ever remove players.
        // And it's going to be really icky if you're not the master.
        // You'll want to know which is your own player, and be able to look it up.

        if active_cell_dweller.maybe_entity.is_none() {
            if let Some(player_id) = client_state.player_id {
                let player = &game_state.players[player_id.0 as usize];
                if let Some(fighter_entity) = player.fighter_entity {
                    // Set our player character as the currently controlled cell dweller.
                    active_cell_dweller.maybe_entity = Some(fighter_entity);
                }
            }
        }

        if client_state.camera_entity.is_none() {
            if let Some(fighter_entity) = active_cell_dweller.maybe_entity {
                // We can only do this after the fighter has been realized.
                // TODO: there's got to be a better pattern for this...
                if let Some(_cell_dweller) = cell_dwellers.get(fighter_entity) {
                    // Create basic third-person following camera.
                    client_state.camera_entity = Some(
                        pk::simple::create_simple_chase_camera(
                            &entities,
                            &updater,
                            fighter_entity,
                            &mut default_camera,
                        )
                    );
                }
            }
        }
    }
}
