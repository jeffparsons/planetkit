use specs;

use globe::{ CellPos, Dir };
use globe::movement::*;

pub struct CellDweller {
    // TODO: make these private and use guts trait pattern to expose them internally.
    // TODO: is this guts pattern worth a separate macro crate of its own?
    pos: CellPos,
    dir: Dir,
}

impl CellDweller {
    pub fn new(pos: CellPos, dir: Dir) -> CellDweller {
        CellDweller {
            pos: pos,
            dir: dir,
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
        move_forward(&mut self.pos, &mut self.dir).unwrap();
    }
}

impl specs::Component for CellDweller {
    type Storage = specs::HashMapStorage<CellDweller>;
}
