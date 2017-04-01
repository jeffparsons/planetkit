use specs;

/// The player character: a shepherd who must find and rescue the sheep
/// that have strayed from his flock and fallen into holes.
pub struct Shepherd {
    pub specs_entity: specs::Entity,
}

impl Shepherd {
    pub fn new(specs_entity: specs::Entity) -> Shepherd {
        Shepherd {
            specs_entity: specs_entity,
        }
    }
}
