use specs;

use globe::chunk::CellPos;

pub struct CellDweller {
    // TODO: make these private and use guts trait pattern to expose them internally.
    // TODO: is this guts pattern worth a separate macro crate of its own?
    cell_pos: CellPos,
}

impl CellDweller {
    pub fn new(cell_pos: CellPos) -> CellDweller {
        CellDweller {
            cell_pos: cell_pos,
        }
    }

    pub fn cell_pos(&self) -> CellPos {
        self.cell_pos
    }

    pub fn set_cell_pos(&mut self, new_pos: CellPos) {
        self.cell_pos = new_pos;
    }

    /// Temporary function for testing until we have actual
    /// movement on the grid. This just adds one to the x-coordinate
    /// of the current position.
    pub fn temp_advance_pos(&mut self) {
        self.cell_pos.x += 1;
    }
}

impl specs::Component for CellDweller {
    type Storage = specs::HashMapStorage<CellDweller>;
}
