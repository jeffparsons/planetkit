use specs;

use super::types::*;

pub struct Spatial {
    pub transform: Iso3,
}

impl Spatial {
    // TODO: explain about hierarchical coordinate systems plan
    pub fn root() -> Self {
        use num_traits::One;
        Spatial {
            transform: Iso3::one(),
        }
    }
}

impl specs::Component for Spatial {
    type Storage = specs::VecStorage<Spatial>;
}
