#[macro_use]
extern crate serde_derive;
#[cfg(test)]
#[macro_use]
extern crate itertools;

use rand::Rng;
use nalgebra as na;

pub mod cell_shape;
pub mod movement;
mod dir;
mod equivalent_points;
mod grid_point2;
mod grid_point3;
mod neighbors;
mod root;

// TODO: be selective in what you export; no wildcards!
pub use self::dir::*;
pub use self::equivalent_points::*;
pub use self::grid_point2::GridPoint2;
pub use self::grid_point3::*;
pub use self::neighbors::*;
pub use self::root::*;

// TODO: Generic!
pub type GridCoord = i64;

/// Generate a random column on the globe.
pub fn random_column<R: Rng>(root_resolution: [GridCoord; 2], rng: &mut R) -> GridPoint2 {
    // TODO: this is a bit dodgy; it isn't uniformly distributed
    // over all points in the world. You should just reject any points
    // that turn out not to be the canonical representation,
    // and try again.
    let root_index: RootIndex = rng.gen_range(0, 5);
    let x: GridCoord = rng.gen_range(0, root_resolution[0]);
    let y: GridCoord = rng.gen_range(0, root_resolution[0]);
    GridPoint2::new(root_index.into(), x, y)
}
