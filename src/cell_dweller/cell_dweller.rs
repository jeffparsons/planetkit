use specs;

use ::types::*;
use globe::{ CellPos, Dir, Spec };
use ::movement::*;

pub struct CellDweller {
    // TODO: make these private and use guts trait pattern to expose them internally.
    // TODO: is this guts pattern worth a separate macro crate of its own?
    pos: CellPos,
    dir: Dir,
    last_turn_bias: TurnDir,
    // Most `CellDweller`s will also be `Spatial`s. Track whether the
    // computed real-space transform has been updated since the globe-space
    // transform was modified so we know when the former is dirty.
    is_real_space_transform_dirty: bool,
    globe_spec: Spec,
    pub seconds_between_moves: TimeDelta,
    pub seconds_until_next_move: TimeDelta,
    pub seconds_between_turns: TimeDelta,
    pub seconds_until_next_turns: TimeDelta,
}

impl CellDweller {
    pub fn new(pos: CellPos, dir: Dir, globe_spec: Spec) -> CellDweller {
        CellDweller {
            pos: pos,
            dir: dir,
            last_turn_bias: TurnDir::Right,
            is_real_space_transform_dirty: true,
            globe_spec: globe_spec,
            // TODO: accept as parameter
            seconds_between_moves: 0.1,
            seconds_until_next_move: 0.0,
            // TODO: accept as parameter
            seconds_between_turns: 0.1,
            seconds_until_next_turns: 0.0,
        }
    }

    pub fn pos(&self) -> CellPos {
        self.pos
    }

    pub fn set_cell_pos(&mut self, new_pos: CellPos) {
        self.pos = new_pos;
        self.is_real_space_transform_dirty = true;
    }

    pub fn dir(&self) -> Dir {
        self.dir
    }

    pub fn step_forward(&mut self) {
        step_forward_and_face_neighbor(
            &mut self.pos,
            &mut self.dir,
            self.globe_spec.root_resolution,
            &mut self.last_turn_bias,
        ).expect("This suggests a bug in `movement` code.");
        self.is_real_space_transform_dirty = true;
    }

    pub fn step_backward(&mut self) {
        step_backward_and_face_neighbor(
            &mut self.pos,
            &mut self.dir,
            self.globe_spec.root_resolution,
            &mut self.last_turn_bias,
        ).expect("This suggests a bug in `movement` code.");
        self.is_real_space_transform_dirty = true;
    }

    pub fn turn(&mut self, turn_dir: TurnDir) {
        turn_by_one_hex_edge(
            &mut self.pos,
            &mut self.dir,
            self.globe_spec.root_resolution,
            turn_dir,
        ).expect("This suggests a bug in `movement` code.");
        self.is_real_space_transform_dirty = true;
    }

    /// Calculate position in real-space.
    fn real_pos(&self) -> Pt3 {
        self.globe_spec.cell_bottom_center(self.pos)
    }

    pub fn is_real_space_transform_dirty(&self) -> bool {
        self.is_real_space_transform_dirty
    }

    // TODO: document responsibilities of caller.
    // TODO: return translation and orientation.
    pub fn get_real_transform_and_mark_as_clean(&mut self) -> Pt3 {
        self.is_real_space_transform_dirty = false;
        self.real_pos()
    }
}

impl specs::Component for CellDweller {
    type Storage = specs::HashMapStorage<CellDweller>;
}
