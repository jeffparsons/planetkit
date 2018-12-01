use slog::Logger;
use specs;
use specs::{Read, ReadStorage, Write, WriteStorage};

use crate::pk::cell_dweller::{CellDweller, CellDwellerMessage, SetPosMessage};
use crate::pk::globe::Globe;
use crate::pk::net::{
    Destination, NetMarker, NodeResource, SendMessage, SendMessageQueue, Transport,
};

use crate::fighter::Fighter;
use crate::game_state::GameState;
use crate::health::Health;
use crate::message::Message;

/// Identifies fighters that have run out of health,
/// awards points to their killer, and respawns the victim.
pub struct DeathSystem {
    log: Logger,
}

impl DeathSystem {
    pub fn new(parent_log: &Logger) -> DeathSystem {
        DeathSystem {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for DeathSystem {
    type SystemData = (
        WriteStorage<'a, Health>,
        WriteStorage<'a, CellDweller>,
        WriteStorage<'a, Globe>,
        ReadStorage<'a, Fighter>,
        Write<'a, GameState>,
        Read<'a, NodeResource>,
        Write<'a, SendMessageQueue<Message>>,
        ReadStorage<'a, NetMarker>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use rand::thread_rng;
        use specs::Join;

        let (
            mut healths,
            mut cell_dwellers,
            mut globes,
            fighters,
            mut game_state,
            node_resource,
            mut send_message_queue,
            net_markers,
        ) = data;

        // Don't try to kill anyone off unless we own the world.
        if !node_resource.is_master {
            return;
        }

        // Find any dead fighters.
        for (cd, fighter, health, net_marker) in
            (&mut cell_dwellers, &fighters, &mut healths, &net_markers).join()
        {
            if health.hp <= 0 {
                // If it was a player that caused them to be harmed,
                // then award a point to that player.
                if let Some(last_damaged_by_player_id) = health.last_damaged_by_player_id {
                    // Victim might have left the game.
                    let players = &mut game_state.players;
                    // Temp hacks to get around multi-borrowing.
                    let victim = players
                        .get(fighter.player_id.0 as usize)
                        .map(|victim_player| (victim_player.id, victim_player.name.clone()));
                    if let Some((victim_id, victim_name)) = victim {
                        // Killer might have left the game.
                        if let Some(killer) = players.get_mut(last_damaged_by_player_id.0 as usize)
                        {
                            if killer.id == victim_id {
                                // Oops. Lost points for a self-kill.
                                killer.points -= 1;
                                info!(self.log, "Fighter killed itself!"; "victim" => &killer.name);
                                info!(self.log, "Killer lost a point"; "new_points" => killer.points);
                            } else {
                                // Yay, you win a point for killing someone!
                                killer.points += 1;
                                info!(self.log, "Fighter killed!"; "victim" => &victim_name, "killer" => &killer.name);
                                info!(self.log, "Killer won a point"; "new_points" => killer.points);
                            }
                        } else {
                            // Don't award any points.
                            info!(self.log, "Fighter killed by disconnected player!");
                        }
                    }
                } else {
                    // Don't award any points.
                    info!(self.log, "Fighter killed by environment!");
                }

                // Re-spawn the player.
                // TODO: actually recreate the entity, rather than doing this.
                // Otherwise we need to find all the components that need
                // to be reset, and that's lame.
                health.hp = 100;

                // Get the associated globe, complaining loudly if we fail.
                // TODO: this same old pattern again.
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

                // TODO: this is copy-pasted from `fighter.rs`. Factor it out somewhere
                // into a function that knows how to spawn replacement fighters.
                let new_fighter_pos = globe
                    .air_above_random_surface_dry_land(
                        &mut thread_rng(),
                        2,  // Min air cells above
                        5,  // Max distance from starting point
                        50, // Max attempts
                    )
                    // TODO: don't panic! Just give up and try again on another tick
                    // if it takes too long on this tick! (Leave the player dead for a while;
                    // we'll probably want to do that soon, anyway: make the screen red for
                    // a couple of seconds while they wait to respawn.)
                    .expect("Oh noes, we took too many attempts to find a decent spawn point!");

                cd.set_grid_point(new_fighter_pos);

                // Tell everyone else that the "respawned" player was been moved.
                send_message_queue.queue.push_back(SendMessage {
                    // In practice, if you're the server this will mean "all clients"
                    // because all of them need to know about the change, and if you're
                    // a client then for now your only peer will be the server.
                    // All of this will obviously need to be revisited if we allow
                    // connecting to multiple servers, or to other non-server peers.
                    destination: Destination::EveryoneElse,
                    game_message: Message::CellDweller(CellDwellerMessage::SetPos(SetPosMessage {
                        entity_id: net_marker.id,
                        new_pos: cd.pos,
                        new_dir: cd.dir,
                        new_last_turn_bias: cd.last_turn_bias,
                    })),
                    transport: Transport::UDP,
                });
            }
        }
    }
}
