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
    // TODO: z
}

pub struct Cell {
    pub material: Material,
}

// TODO: make this an actual voxmap.
pub struct Chunk {
    pub origin: CellPos,
    // Sorted by (z, y, x).
    pub cells: Vec<Cell>,
}
