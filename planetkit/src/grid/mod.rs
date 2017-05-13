use rand::Rng;

mod root;
mod grid_point2;
pub mod cell_shape;
mod cell_pos;
mod neighbors;
mod dir;

// TODO: be selective in what you export; no wildcards!
pub use self::root::*;
pub use self::grid_point2::GridPoint2;
pub use self::cell_pos::*;
pub use self::neighbors::*;
pub use self::dir::*;

pub type IntCoord = i64;

/// Generate a random column on the globe.
pub fn random_column<R: Rng>(
    root_resolution: [IntCoord; 2],
    rng: &mut R,
) -> GridPoint2 {
    // TODO: this is a bit dodgy; it isn't uniformly distributed
    // over all points in the world.
    let root_index: RootIndex = rng.gen_range(0, 5);
    let x: IntCoord = rng.gen_range(0, root_resolution[0]);
    let y: IntCoord = rng.gen_range(0, root_resolution[0]);
    GridPoint2::new(root_index.into(), x, y)
}
