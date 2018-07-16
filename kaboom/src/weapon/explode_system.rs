use specs;
use specs::{Read, Write, Entities, ReadStorage, WriteStorage};
use slog::Logger;

use pk::types::*;
use pk::Spatial;
use pk::net::NodeResource;
use pk::physics;

use ::health::Health;
use super::grenade::Grenade;

pub struct ExplodeSystem {
    log: Logger,
}

impl ExplodeSystem {
    pub fn new(
        parent_log: &Logger,
    ) -> ExplodeSystem {
        ExplodeSystem {
            log: parent_log.new(o!()),
        }
    }
}

impl<'a> specs::System<'a> for ExplodeSystem {
    type SystemData = (
        Read<'a, TimeDeltaResource>,
        Entities<'a>,
        WriteStorage<'a, Grenade>,
        WriteStorage<'a, Health>,
        ReadStorage<'a, Spatial>,
        Read<'a, NodeResource>,
        Read<'a, physics::WorldResource>,
        ReadStorage<'a, physics::RigidBody>,
        ReadStorage<'a, physics::Collider>,
        Write<'a, physics::RemoveBodyQueue>,
        Write<'a, physics::RemoveColliderQueue>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;
        use pk::SpatialStorage;

        let (
            dt,
            entities,
            mut grenades,
            mut healths,
            spatials,
            node_resource,
            world_resource,
            rigid_bodies,
            colliders,
            mut remove_body_queue_resource,
            mut remove_collider_queue_resource,
        ) = data;

        let nphysics_world = &world_resource.world;
        let remove_body_queue = &mut remove_body_queue_resource.queue;
        let remove_collider_queue = &mut remove_collider_queue_resource.queue;

        for (grenade_entity, grenade, rigid_body, collider) in (&*entities, &mut grenades, &rigid_bodies, &colliders).join() {
            // Count down each grenade's timer, and remove it if
            // it's been alive too long.
            grenade.time_to_live_seconds -= dt.0;
            grenade.time_lived_seconds += dt.0;

            // Check if the grenade had collided with anything.
            // TODO: at the time of writing, we're not actually
            // using nphysics to apply velocity, but just updating
            // the grenade's position from planetkit's side.
            // This means that it could skip through the chunk
            // without touching it if it goes fast enough. 😅
            // Fix ASAP after testing initial hacks. :)
            //
            // Give a little grace period after firing, becase we
            // currently fire the grenade from the player's feet,
            // and we don't want it to immediately explode on the
            // ground beneath them. Even if we fix it to fire higher,
            // we probably don't want it to explode on the firing player,
            // so we'll need to do something a bit more subtle than this
            // eventually!
            //
            // TODO: Replace with sensor... you don't want this
            // thing to bounce... or DO you? :P Maybe that'll be
            // a setting: number of bounces. It'll be way more fun like that.
            use ncollide3d::events::ContactEvent;
            let did_hit_something = grenade.time_lived_seconds > 0.2 &&
                nphysics_world.contact_events()
                .iter()
                .any(|contact_event| {
                    match contact_event {
                        ContactEvent::Started(a, b) => {
                            // Collision could be either way around...?
                            *a == collider.collider_handle
                            ||
                            *b == collider.collider_handle
                        },
                        _ => false,
                    }
                });

            if did_hit_something || grenade.time_to_live_seconds <= 0.0 {
                debug!(self.log, "Kaboom!"; "did_hit_something" => did_hit_something);

                entities.delete(grenade_entity).expect("Wrong entity generation!");

                // Queue it for removal from physics world.
                // TODO: These are hacks until Specs addresses reading
                // the data of removed components. (Presumably some extension
                // to the existing FlaggedStorage where you indicate that
                // you want the channel to carry full component data
                // with each event?)
                use pk::physics::{RemoveBodyMessage, RemoveColliderMessage};
                remove_body_queue.push_back(RemoveBodyMessage {
                    handle: rigid_body.body_handle,
                });
                remove_collider_queue.push_back(RemoveColliderMessage {
                    handle: collider.collider_handle,
                });

                // NOTE: Hacks until we have saveload and figure out how to do networking better.
                if !node_resource.is_master {
                    continue;
                }

                // Damage anything nearby that can take damage.
                for (living_thing_entity, health, _spatial) in (&*entities, &mut healths, &spatials).join() {
                    let relative_transform = spatials.a_relative_to_b(living_thing_entity, grenade_entity);
                    let distance_squared = relative_transform.translation.vector.norm_squared();
                    let blast_radius_squared = 2.5 * 2.5;

                    if distance_squared <= blast_radius_squared {
                        health.hp -= 100;
                        health.last_damaged_by_player_id = Some(grenade.fired_by_player_id);
                        debug!(self.log, "Damaged something!");
                    }
                }
            }
        }
    }
}
