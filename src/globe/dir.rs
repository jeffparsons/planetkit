pub type DirIndex = u8;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Dir {
    pub index: DirIndex,
}

impl Dir {
    pub fn new(index: DirIndex) -> Dir {
        Dir {
            index: index
        }
    }

    /// Returns `true` if `self` points toward an immediately
    /// adjacent cell, or equivalently toward an edge of the
    /// current cell.
    ///
    /// Assumes this is in the context of a hexagonal cell --
    /// i.e. not one of the 12 pentagons in the world.
    /// If you need to ask an equivalent question when you might
    /// be in a pentagonal cell, then first rebase your
    /// `(Pos, Dir)` onto a root quad that `self` points into,
    /// and then the relevant part of the current cell will
    /// then be equivalent to a hexagon for this purpose.
    pub fn points_at_hex_edge(&self) -> bool {
        // On a hexagonal cell, any even direction index
        // points to an edge.
        self.index % 2 == 0
    }
}

impl From<DirIndex> for Dir {
    fn from(dir_index: DirIndex) -> Dir {
        Dir::new(dir_index)
    }
}
