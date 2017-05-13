use std::ops::{ Deref, DerefMut };

use super::{ IntCoord, GridPoint2, Root, RootIndex };

// TODO: rename to GridPoint3
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct CellPos {
    pub rxy: GridPoint2,
    pub z: IntCoord,
}

impl CellPos {
    pub fn new(root: Root, x: IntCoord, y: IntCoord, z: IntCoord) -> CellPos {
        CellPos {
            rxy: GridPoint2::new(root, x, y),
            z: z,
        }
    }

    pub fn with_root(&self, new_root_index: RootIndex) -> Self {
        let mut new_point = *self;
        new_point.rxy.root.index = new_root_index;
        new_point
    }

    pub fn with_x(&self, new_x: IntCoord) -> Self {
        let mut new_point = *self;
        new_point.rxy.x = new_x;
        new_point
    }

    pub fn with_y(&self, new_y: IntCoord) -> Self {
        let mut new_point = *self;
        new_point.rxy.y = new_y;
        new_point
    }

    pub fn with_z(&self, new_z: IntCoord) -> Self {
        let mut new_point = *self;
        new_point.z = new_z;
        new_point
    }
}

/// Wrapper type around a `Pos` that is known to be expressed
/// in its owning root quad.
///
/// Note that this does not save you from accidentally using
/// positions from multiple incompatible `Globe`s with different
/// resolutions.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct PosInOwningRoot {
    pos: CellPos,
}

impl Into<CellPos> for PosInOwningRoot {
    fn into(self) -> CellPos {
        self.pos
    }
}

impl PosInOwningRoot {
    // Returns a position equivalent to `pos`,
    // but in whatever root owns the data for `pos`.
    //
    // The output will only ever differ from the input
    // if `pos` is on the edge of a root quad.
    //
    // Behaviour is undefined (nonsense result or panic)
    // if `pos` lies beyond the edges of its root.
    pub fn new(pos: CellPos, resolution: [IntCoord; 2]) -> PosInOwningRoot {
        debug_assert!(pos.z >= 0);

        // Here is the pattern of which root a cell belongs to.
        //
        // Note how adacent roots neatly slot into each other's
        // non-owned cells when wrapped around the globe.
        //
        // Also note the special cases for north and south poles;
        // they don't fit neatly into the general pattern.
        //
        // In the diagram below, each circle represents a hexagon
        // in a voxmap shell. Filled circles belong to the root,
        // and empty circles belong to an adjacent root.
        //
        //   Root 0   Roots 1, 2, 3   Root 4
        //   ------   -------------   ------
        //
        //      ●           ◌           ◌
        //     ◌ ●         ◌ ●         ◌ ●
        //    ◌ ● ●       ◌ ● ●       ◌ ● ●
        //   ◌ ● ● ●     ◌ ● ● ●     ◌ ● ● ●
        //  ◌ ● ● ● ●   ◌ ● ● ● ●   ◌ ● ● ● ●
        //   ◌ ● ● ● ●   ◌ ● ● ● ●   ◌ ● ● ● ●
        //    ◌ ● ● ● ●   ◌ ● ● ● ●   ◌ ● ● ● ●
        //     ◌ ● ● ● ●   ◌ ● ● ● ●   ◌ ● ● ● ●
        //      ◌ ● ● ● ●   ◌ ● ● ● ●   ◌ ● ● ● ●
        //       ◌ ● ● ●     ◌ ● ● ●     ◌ ● ● ●
        //        ◌ ● ●       ◌ ● ●       ◌ ● ●
        //         ◌ ●         ◌ ●         ◌ ●
        //          ◌           ◌           ●
        //
        let end_x = resolution[0];
        let end_y = resolution[1];
        let half_y = resolution[1] / 2;

        // Special cases for north and south poles
        let pos_in_owning_root = if pos.x == 0 && pos.y == 0 {
            // North pole
            CellPos::new(
                // First root owns north pole.
                0.into(),
                0,
                0,
                pos.z,
            )
        } else if pos.x == end_x && pos.y == end_y {
            // South pole
            CellPos::new(
                // Last root owns south pole.
                4.into(),
                end_x,
                end_y,
                pos.z,
            )
        } else if pos.y == 0 {
            // Roots don't own their north-west edge;
            // translate to next root's north-east edge.
            CellPos::new(
                pos.root.next_west(),
                0,
                pos.x,
                pos.z,
            )
        } else if pos.x == end_x && pos.y < half_y {
            // Roots don't own their mid-west edge;
            // translate to the next root's mid-east edge.
            CellPos::new(
                pos.root.next_west(),
                0,
                half_y + pos.y,
                pos.z,
            )
        } else if pos.x == end_x {
            // Roots don't own their south-west edge;
            // translate to the next root's south-east edge.
            CellPos::new(
                pos.root.next_west(),
                pos.y - half_y,
                end_y,
                pos.z,
            )
        } else {
            // `pos` is either on an edge owned by its root,
            // or somewhere in the middle of the root.
            pos
        };

        PosInOwningRoot {
            pos: pos_in_owning_root
        }
    }

    /// Set z-coordinate of underlying `Pos`.
    ///
    /// Note that this is the one safe axis to operate
    /// on without knowing the globe resolution.
    pub fn set_z(&mut self, new_z: IntCoord) {
        self.pos.z = new_z;
    }
}

impl<'a> PosInOwningRoot {
    pub fn pos(&'a self) -> &'a CellPos {
        &self.pos
    }
}

// Evil tricks to allow access to GridPoint2 fields from `self.rxy`
// as if they belong to `Self`.
impl Deref for CellPos {
    type Target = GridPoint2;

    fn deref(&self) -> &GridPoint2 {
        &self.rxy
    }
}

impl DerefMut for CellPos {
    fn deref_mut(&mut self) -> &mut GridPoint2 {
        &mut self.rxy
    }
}
