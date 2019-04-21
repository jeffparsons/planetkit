use specs;

/// Objects that have mass, and are therefore affected by gravity.
///
/// Assumed to also be a `Velocity`. (That's the component its
/// acceleration will be applied to.)
pub struct Mass {
    // TODO: Store its actual mass? For now that is irrelevant.
}

impl Mass {
    pub fn new() -> Mass {
        Mass {}
    }
}

impl specs::Component for Mass {
    type Storage = specs::VecStorage<Mass>;
}

impl Default for Mass {
    fn default() -> Self {
        Self::new()
    }
}
