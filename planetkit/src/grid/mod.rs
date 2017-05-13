use rand::Rng;

mod root;
pub mod cell_shape;
mod cell_pos;
mod neighbors;
mod dir;

// TODO: be selective in what you export; no wildcards!
pub use self::root::*;
pub use self::cell_pos::*;
pub use self::neighbors::*;
pub use self::dir::*;

pub type IntCoord = i64;

/// Generate a random column on the globe.
///
/// The position returned will always have a `z`-value of 0.
pub fn random_column<R: Rng>(
    root_resolution: [IntCoord; 2],
    rng: &mut R,
) -> CellPos {
    // TODO: this is a bit dodgy; it isn't uniformly distributed
    // over all points in the world.
    let root_index: RootIndex = rng.gen_range(0, 5);
    let x: IntCoord = rng.gen_range(0, root_resolution[0]);
    let y: IntCoord = rng.gen_range(0, root_resolution[0]);
    CellPos {
        root: root_index.into(),
        x: x,
        y: y,
        z: 0,
    }
}
