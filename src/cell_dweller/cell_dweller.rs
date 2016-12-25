use specs;

use ::types::*;
use globe::{ CellPos, Dir, Spec };
use globe::movement::*;

pub struct CellDweller {
    // TODO: make these private and use guts trait pattern to expose them internally.
    // TODO: is this guts pattern worth a separate macro crate of its own?
    pos: CellPos,
    dir: Dir,
    globe_spec: Spec,
}

impl CellDweller {
    pub fn new(pos: CellPos, dir: Dir, globe_spec: Spec) -> CellDweller {
        CellDweller {
            pos: pos,
            dir: dir,
            globe_spec: globe_spec,
        }
    }

    pub fn pos(&self) -> CellPos {
        self.pos
    }

    pub fn set_cell_pos(&mut self, new_pos: CellPos) {
        self.pos = new_pos;
    }

    /// Temporary function for testing until we have actual
    /// movement on the grid. This just adds one to the x-coordinate
    /// of the current position.
    pub fn temp_advance_pos(&mut self) {
        move_forward(
            &mut self.pos,
            &mut self.dir,
            self.globe_spec.root_resolution,
        ).unwrap();
    }

    /// Calculate position in real-space.
    pub fn real_pos(&self) -> Pt3 {
        self.globe_spec.cell_bottom_center(self.pos)
    }
}

impl specs::Component for CellDweller {
    type Storage = specs::HashMapStorage<CellDweller>;
}
