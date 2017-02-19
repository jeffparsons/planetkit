// See `triangles.rs` for discussion about the approach used
// throughout the module, and the list of all triangles used.

mod triangles;
mod transform;
mod step;
mod turn;
mod util;

#[cfg(test)]
mod tests;

// TODO: figure out how to encourage use of the "good" functions,
// while still exposing the "raw" ones for people who really want them.
// Consider something like session types.

pub use self::step::{
    move_forward,
    step_forward_and_face_neighbor,
    step_backward_and_face_neighbor,
};
pub use self::turn::{
    TurnDir,
    turn_left_by_one_hex_edge,
    turn_right_by_one_hex_edge,
    turn_by_one_hex_edge,
    turn_around_and_face_neighbor,
};
pub use self::util::{
    adjacent_pos_in_dir,
    is_pentagon,
};
