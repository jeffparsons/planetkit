// See `triangles.rs` for discussion about the approach used
// throughout the module, and the list of all triangles used.

mod triangles;
mod transform;
mod step;
mod turn;
mod util;

#[cfg(test)]
mod test;

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
