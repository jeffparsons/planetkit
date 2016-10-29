use globe::{ IntCoord, Root };

#[derive(PartialEq, Eq)]
pub enum Material {
    Air,
    Dirt,
}

// TODO: we should actually have multiple different
// kinds of Voxmaps. "Chunk" should refer to the coarse
// entity that owns everything related to a conveniently
// sized partition of the world that would be loaded and
// unloaded into the world as a unit.

#[derive(Clone, Copy)]
pub struct CellPos {
    pub root: Root,
    pub x: IntCoord,
    pub y: IntCoord,
    pub z: IntCoord,
}

pub struct Cell {
    pub material: Material,
}

// TODO: make this an actual voxmap.
pub struct Chunk {
    pub origin: CellPos,
    pub resolution: [IntCoord; 3],
    // Sorted by (z, y, x).
    pub cells: Vec<Cell>,
}

impl<'a> Chunk {
    // Panics if given coordinates of a cell we don't have data for.
    pub fn cell(&'a self, pos: CellPos) -> &'a Cell {
        let local_x = pos.x - self.origin.x;
        let local_y = pos.y - self.origin.y;
        let cell_i = (local_y * self.resolution[0] + local_x) as usize;
        &self.cells[cell_i]
    }
}
