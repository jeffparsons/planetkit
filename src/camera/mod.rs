use specs;

/// Default camera to be used by render system.
///
/// This is intended to be used as a Specs resource.
pub struct DefaultCamera {
    pub camera_entity: specs::Entity,
}
