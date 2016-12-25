use specs;

use super::types::*;

pub struct Spatial {
    pub pos: Pt3,
}

impl Spatial {
    // TODO: explain about hierarchical coordinate systems plan
    pub fn root() -> Self {
        use na::Origin;
        Spatial {
            pos: Pt3::origin(),
        }
    }
}

impl specs::Component for Spatial {
    type Storage = specs::VecStorage<Spatial>;
}
