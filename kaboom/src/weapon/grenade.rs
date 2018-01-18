use specs::{
    self,
    Entity,
    Fetch,
    LazyUpdate,
    Entities,
    ReadStorage,
    WriteStorage
};
use slog::Logger;

use pk::types::*;
use pk::render;
use pk::cell_dweller::CellDweller;
use pk::physics::Velocity;
use pk::physics::Mass;
use pk::Spatial;

/// Velocity relative to some parent entity.
///
/// Assumed to also be a `Spatial`. (That's where its parent
/// reference is stored, and there's no meaning to velocity
/// without position.)
pub struct Grenade {
    pub time_to_live_seconds: f64,
}

impl Grenade {
    pub fn new() -> Grenade {
        Grenade {
            time_to_live_seconds: 1.5,
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
    updater: &Fetch<LazyUpdate>,
    cell_dwellers: &ReadStorage<CellDweller>,
    cell_dweller_entity: Entity,
    spatials: &WriteStorage<Spatial>,
    log: &Logger,
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

    // Build the entity.
    let entity = entities.create();
    updater.insert(entity, bullet_visual);
    updater.insert(entity, bullet_spatial);
    updater.insert(entity, bullet_velocity);
    updater.insert(entity, Mass{});
    updater.insert(entity, Grenade::new());
}
