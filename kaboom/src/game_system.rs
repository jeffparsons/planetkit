use specs;
use specs::{WriteStorage, Fetch, FetchMut, LazyUpdate, Entities};
use slog::Logger;

use pk;
use pk::globe::Globe;
use pk::cell_dweller::{CellDweller, ActiveCellDweller};
use pk::camera::DefaultCamera;
use pk::net::{PeerId, NetworkPeers, Destination, Transport, SendMessageQueue, SendMessage};

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
        GameState::ensure(world);
        ClientState::ensure(world);
        player::RecvMessageQueue::ensure(world);

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
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
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
        ) = data;

        while let Some(message) = player_recv_message_queue.queue.pop_front() {
            match message.game_message {
                PlayerMessage::NewPlayer(player_id) => {
                    // Add the new player to our list.

                    // Master should never be told about new players.
                    // TODO: don't explode, just log a loud error,
                    // and kick the client who sent the bad message.
                    // You need a pattern for this.
                    assert!(!client_state.is_master);

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
            }
        }

        // If we are the master, but we don't yet know what our player is,
        // then insert a new player for us now. We'll hear about it on the
        // next tick, and register it as our own.
        if client_state.is_master && client_state.player_id.is_none() {
            self.create_and_broadcast_player(&mut game_state, &mut send_message_queue, PeerId(0));
        }

        // If there are any new network peers, then pop them off
        // and maybe do something with them.
        while let Some(new_peer_id) = network_peers.new_peers.pop_front() {
            // As a client, we don't care; we just want to clean out the list.
            if client_state.is_master {
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

                // Create a new player for that peer.
                self.create_and_broadcast_player(&mut game_state, &mut send_message_queue, new_peer_id);
            }

            // TODO: instead first just create a player for them,
            // then tell them about all existing players,
            // and then which player is theirs.
        }

        // TODO: eventually only the server should create this,
        // and then describe it to clients.
        if game_state.globe_entity.is_none() {
            // Create the globe first, because we'll need it to figure out where
            // to place the player character.
            game_state.globe_entity = Some(
                planet::create(&entities, &updater)
            );
        }

        // Create a new character for each new player.
        if client_state.is_master {
            if let Some(globe_entity) = game_state.globe_entity {
                // We can only do this after the globe has been realized.
                if let Some(mut globe) = globes.get_mut(globe_entity) {
                    while let Some(player_id) = game_state.new_players.pop_front() {
                        info!(self.log, "Found a new player; making a fighter for them"; "player_id" => format!("{:?}", player_id));

                        // Create the player character.
                        // TODO: figure out how you're doing entity UUIDs;
                        // you'll need to tell your network peers about it.
                        let fighter_entity = fighter::create(
                            &entities,
                            &updater,
                            globe_entity,
                            &mut globe,
                        );

                        let player = &mut game_state.players[player_id.0 as usize];
                        player.fighter_entity = Some(fighter_entity);

                        // TODO: tell all network peers about the
                        // new entity, and who owns it. Then whoever
                        // owns it will be able to control it!
                    }
                }
            }
        }

        // TODO: index in a HashMap or something by PlayerId?
        // At the moment you can't ever remove players.
        // And it's going to be really icky if you're not the master.
        // You'll want to know which is your own player, and be able to look it up.

        // TODO: this needs to become about looking up your own player.

        if client_state.is_master && active_cell_dweller.maybe_entity.is_none() {
            if let Some(fighter_entity) = game_state.players[0].fighter_entity {
                // Set our player character as the currently controlled cell dweller.
                active_cell_dweller.maybe_entity = Some(fighter_entity);
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
