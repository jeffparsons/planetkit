use ncollide3d::shape::ShapeHandle;
use slog::Logger;
use specs::{self, Entities, Entity, LazyUpdate, Read, ReadStorage, Write};

use crate::pk::cell_dweller::CellDweller;
use crate::pk::physics::Mass;
use crate::pk::physics::Velocity;
use crate::pk::physics::{Collider, RigidBody, WorldResource};
use crate::pk::render;
use crate::pk::types::*;
use crate::pk::Spatial;

use crate::player::PlayerId;

pub struct Grenade {
    pub time_to_live_seconds: f64,
    // So that we don't accidentally make it explode
    // immediately after launching. This is a bit of a hack
    // because we launch it from the player's feet at the moment.
    //
    // But we probably want to support this in some form.
    // I'm thinking being able to set the max number of bounces
    // before you fire (like setting the grenade timer in Worms),
    // and have it ignore bounces that happen in rapid succession,
    // or immediately after firing.
    pub time_lived_seconds: f64,
    pub fired_by_player_id: PlayerId,
}

impl Grenade {
    pub fn new(fired_by_player_id: PlayerId) -> Grenade {
        Grenade {
            // It should usually hit terrain before running out
            // of time. But if you fire from a tall hill,
            // it might explode in the air.
            time_to_live_seconds: 3.0,
            time_lived_seconds: 0.0,
            fired_by_player_id: fired_by_player_id,
        }
    }
}

impl specs::Component for Grenade {
    type Storage = specs::HashMapStorage<Grenade>;
}

/// Spawn a grenade travelling up and forward away from the player.
pub fn shoot_grenade(
    entities: &Entities<'_>,
    updater: &Read<'_, LazyUpdate>,
    cell_dwellers: &ReadStorage<'_, CellDweller>,
    cell_dweller_entity: Entity,
    spatials: &ReadStorage<'_, Spatial>,
    log: &Logger,
    fired_by_player_id: PlayerId,
    world_resource: &mut Write<'_, WorldResource>,
) {
    // Make visual appearance of bullet.
    // For now this is just an axes mesh.
    let mut bullet_visual = render::Visual::new_empty();
    bullet_visual.proto_mesh = Some(render::make_axes_mesh());

    let cd = cell_dwellers
        .get(cell_dweller_entity)
        .expect("Someone deleted the controlled entity's CellDweller");
    let cd_spatial = spatials
        .get(cell_dweller_entity)
        .expect("Someone deleted the controlled entity's Spatial");
    // Get the associated globe entity, complaining loudly if we fail.
    let globe_entity = match cd.globe_entity {
        Some(globe_entity) => globe_entity,
        None => {
            warn!(
                log,
                "There was no associated globe entity or it wasn't actually a Globe! Can't proceed!"
            );
            return;
        }
    };
    // Put bullet where player is.
    let bullet_spatial = Spatial::new(globe_entity, cd_spatial.local_transform());

    // Shoot the bullet slightly up and away from us.
    //
    // (TODO: turn panic into error log.)
    //
    // NOTE: the `unwrap` here is not the normal meaning of unwrap;
    // in this case it is a totally innocuous function for extracting
    // the interior value of a unit vector.
    let dir = &cd_spatial.local_transform().rotation;
    let cd_relative_velocity = (Vec3::z_axis().unwrap() + Vec3::y_axis().unwrap()) * 7.0;
    let bullet_velocity = Velocity::new(dir * cd_relative_velocity);

    // Add a small ball for the grenade to the physics world.
    use ncollide3d::shape::Ball;
    use nphysics3d::object::Material;
    let ball = Ball::<Real>::new(0.1);
    let ball_handle = ShapeHandle::new(ball);
    let world = &mut world_resource.world;

    // Set up rigid body.
    use nphysics3d::volumetric::Volumetric;
    let inertia = ball_handle.inertia(1.0);
    let center_of_mass = ball_handle.center_of_mass();
    let pos = cd_spatial.local_transform();
    let rigid_body_handle = world.add_rigid_body(pos, inertia, center_of_mass);

    let collider_handle = world.add_collider(
        0.01 as Real, // TODO: What's appropriate?
        ball_handle.clone(),
        rigid_body_handle.clone(),
        Iso3::identity(),
        Material::default(),
    );

    // Build the entity.
    let entity = entities.create();
    updater.insert(entity, bullet_visual);
    updater.insert(entity, bullet_spatial);
    updater.insert(entity, bullet_velocity);
    updater.insert(entity, Mass {});
    updater.insert(entity, Grenade::new(fired_by_player_id));
    updater.insert(entity, Collider::new(collider_handle));
    updater.insert(entity, RigidBody::new(rigid_body_handle));
}
