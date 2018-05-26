use shred::{self, FetchMut};
use specs::World;

// TODO: use for Systems, too?
// Yes, that'll be pretty neat.
// They can get whatever inputs they need from _resources_.
// If all bounds can be satisfied, this could include loggers, etc. :)
//
// Noooop! :) There's a thing for systems upstream now.
// So just adopt that.

/// `Resource`s that know how to ensure their existence
/// using only a reference to a `World`.
pub trait AutoResource : shred::Resource + Sized {
    /// Ensure the given resource exists in the world.
    /// Returns the resource for writing; this should
    /// only be used during initialization, so contention
    /// shouldn't be an issue.
    ///
    /// Cyclic dependencies will result in a panic.
    fn ensure(world: &mut World) -> FetchMut<Self> {
        if !world.res.has_value::<Self>() {
            let resource = Self::new(world);
            world.add_resource(resource);
        }
        world.write_resource::<Self>()
    }

    fn new(world: &mut World) -> Self;
}
