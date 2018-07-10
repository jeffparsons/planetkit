use specs::{
    self,
    Entity,
    Read,
    Write,
    LazyUpdate,
    Entities,
    ReadStorage,
};
use slog::Logger;
use ncollide3d::shape::ShapeHandle;
use nphysics3d::object::{ColliderHandle, BodyHandle};

use pk::types::*;
use pk::render;
use pk::cell_dweller::CellDweller;
use pk::physics::Velocity;
use pk::physics::Mass;
use pk::Spatial;
use pk::nphysics::WorldResource;

use ::player::PlayerId;

/// Velocity relative to some parent entity.
///
/// Assumed to also be a `Spatial`. (That's where its parent
/// reference is stored, and there's no meaning to velocity
/// without position.)
pub struct Grenade {
    pub time_to_live_seconds: f64,
    // So that we don't accidentally make it explode
    // immediately after launching. This is a bit of a hack
    // because we launch it from the player's feet at the moment.
    pub time_lived_seconds: f64,
    pub fired_by_player_id: PlayerId,
    // Physics stuff
    pub collider_handle: ColliderHandle,
    pub body_handle: BodyHandle,
}

impl Grenade {
    pub fn new(fired_by_player_id: PlayerId, collider_handle: ColliderHandle, body_handle: BodyHandle) -> Grenade {
        Grenade {
            // It should usually hit terrain before running out
            // of time. But if you fire from a tall hill,
            // it might explode in the air.
            time_to_live_seconds: 3.0,
            time_lived_seconds: 0.0,
            fired_by_player_id: fired_by_player_id,
            collider_handle: collider_handle,
            body_handle: body_handle,
        }
    }
}

impl specs::Component for Grenade {
    // TODO: more appropriate storage?
    type Storage = specs::VecStorage<Grenade>;
}

/// Spawn a grenade travelling up and forward away from the player.
pub fn shoot_grenade(
    entities: &Entities,
    updater: &Read<LazyUpdate>,
    cell_dwellers: &ReadStorage<CellDweller>,
    cell_dweller_entity: Entity,
    spatials: &ReadStorage<Spatial>,
    log: &Logger,
    fired_by_player_id: PlayerId,
    world_resource: &mut Write<WorldResource>,
) {
    // Make visual appearance of bullet.
    // For now this is just an axes mesh.
    let mut bullet_visual = render::Visual::new_empty();
    bullet_visual.proto_mesh = Some(render::make_axes_mesh());

    let cd = cell_dwellers.get(cell_dweller_entity).expect(
        "Someone deleted the controlled entity's CellDweller",
    );
    let cd_spatial = spatials.get(cell_dweller_entity).expect(
        "Someone deleted the controlled entity's Spatial",
    );
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
    let bullet_spatial = Spatial::new(
        globe_entity,
        cd_spatial.local_transform(),
    );

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

    // TODO: should this just be a sensor, not a collider??
    // Because the grenade doesn't need to be able to bounce.
    // (Or would that be even cooler; if you can _choose_
    // when you fire it? :D)

    // Build the entity.
    let entity = entities.create();
    updater.insert(entity, bullet_visual);
    updater.insert(entity, bullet_spatial);
    updater.insert(entity, bullet_velocity);
    updater.insert(entity, Mass{});
    updater.insert(entity, Grenade::new(fired_by_player_id, collider_handle, rigid_body_handle));
}
