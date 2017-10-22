use specs;

use ::AutoResource;

/// Default camera to be used by render system.
///
/// This is intended to be used as a Specs resource.
pub struct DefaultCamera {
    pub camera_entity: Option<specs::Entity>,
}

impl AutoResource for DefaultCamera {
	fn new(_world: &mut specs::World) -> DefaultCamera {
		DefaultCamera {
			camera_entity: None,
		}
	}
}
