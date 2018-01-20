use specs;

/// Health points, which can be depleted by incurring damage.
///
/// Health may go below zero if an damage causes greater than
/// the remaining points. It is up to specific games whether to
/// allow incurring further damage when health is already at or
/// below zero.
pub struct Health {
    pub hp: i32,
}

impl Health {
    pub fn new(initial_hp: i32) -> Health {
        Health {
            hp: initial_hp,
        }
    }
}

impl specs::Component for Health {
    type Storage = specs::VecStorage<Health>;
}
