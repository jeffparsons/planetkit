use globe::{ IntCoord, Root };

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
    // For now we're just storing the elevation
    // at each X/Y position. That should all be cached
    // somewhere else, and this should have actual voxel data.
    pub height: f64,
}

// TODO: make this an actual voxmap.
pub struct Chunk {
    pub origin: CellPos,
    // Sorted by (z, y, x).
    pub cells: Vec<Cell>,
}
