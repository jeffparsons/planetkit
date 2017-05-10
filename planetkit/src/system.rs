use specs;

/// Perform any system initialisation that requires mutable access
/// to the `World`, e.g., adding new resources.
pub trait System<T> : specs::System<T> {
    fn init(&mut self, world: &mut specs::World);
}
