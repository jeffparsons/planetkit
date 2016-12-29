// See `triangles.rs` for discussion about the approach used
// throughout the module, and the list of all triangles used.

mod triangles;
mod transform;
mod step;
mod turn;
mod util;

#[cfg(test)]
mod test;

pub use self::step::move_forward;
pub use self::turn::{
    turn_left_by_one_hex_edge,
    turn_right_by_one_hex_edge,
};
