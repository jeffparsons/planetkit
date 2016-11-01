use globe::{ IntCoord, Root };

#[derive(PartialEq, Eq)]
pub enum Material {
    Air,
    Dirt,
    Water,
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

// Stores from (0, 0) to (resolution, resolution) _inclusive_.
//
// TODO: copy storage layout and cell ownership rules from:
// <http://kiwi.atmos.colostate.edu/BUGS/geodesic/text.html>.
// They seem to have a pretty good grasp on these things. :)
pub struct Chunk {
    pub origin: CellPos,
    pub resolution: [IntCoord; 3],
    // Sorted by (z, y, x).
    pub cells: Vec<Cell>,
}

impl<'a> Chunk {
    // Panics if given coordinates of a cell we don't have data for.
    //
    // TODO: _store_ more information to make lookups cheaper.
    pub fn cell(&'a self, pos: CellPos) -> &'a Cell {
        let local_x = pos.x - self.origin.x;
        let local_y = pos.y - self.origin.y;
        let local_z = pos.z - self.origin.z;
        let cell_i =
            local_z * (self.resolution[0] + 1) * (self.resolution[1] + 1) +
            local_y * (self.resolution[0] + 1) +
            local_x;
        &self.cells[cell_i as usize]
    }
}
