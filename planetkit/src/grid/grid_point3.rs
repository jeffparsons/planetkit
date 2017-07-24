use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};

use super::{GridCoord, GridPoint2, Root, RootIndex};

// TODO: rename to GridPoint3
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct GridPoint3 {
    pub rxy: GridPoint2,
    pub z: GridCoord,
}

impl GridPoint3 {
    pub fn new(root: Root, x: GridCoord, y: GridCoord, z: GridCoord) -> GridPoint3 {
        GridPoint3 {
            rxy: GridPoint2::new(root, x, y),
            z: z,
        }
    }

    pub fn with_root(&self, new_root_index: RootIndex) -> Self {
        let mut new_point = *self;
        new_point.rxy.root.index = new_root_index;
        new_point
    }

    pub fn with_x(&self, new_x: GridCoord) -> Self {
        let mut new_point = *self;
        new_point.rxy.x = new_x;
        new_point
    }

    pub fn with_y(&self, new_y: GridCoord) -> Self {
        let mut new_point = *self;
        new_point.rxy.y = new_y;
        new_point
    }

    pub fn with_z(&self, new_z: GridCoord) -> Self {
        let mut new_point = *self;
        new_point.z = new_z;
        new_point
    }
}

/// Wrapper type around a `GridPoint3` that is known to be expressed
/// in its owning root quad.
///
/// Note that this does not save you from accidentally using
/// positions from multiple incompatible `Globe`s with different
/// resolutions.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct PosInOwningRoot {
    pos: GridPoint3,
}

impl Into<GridPoint3> for PosInOwningRoot {
    fn into(self) -> GridPoint3 {
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
    pub fn new(pos: GridPoint3, resolution: [GridCoord; 2]) -> PosInOwningRoot {
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
            GridPoint3::new(
                // First root owns north pole.
                0.into(),
                0,
                0,
                pos.z,
            )
        } else if pos.x == end_x && pos.y == end_y {
            // South pole
            GridPoint3::new(
                // Last root owns south pole.
                4.into(),
                end_x,
                end_y,
                pos.z,
            )
        } else if pos.y == 0 {
            // Roots don't own their north-west edge;
            // translate to next root's north-east edge.
            GridPoint3::new(pos.root.next_west(), 0, pos.x, pos.z)
        } else if pos.x == end_x && pos.y < half_y {
            // Roots don't own their mid-west edge;
            // translate to the next root's mid-east edge.
            GridPoint3::new(pos.root.next_west(), 0, half_y + pos.y, pos.z)
        } else if pos.x == end_x {
            // Roots don't own their south-west edge;
            // translate to the next root's south-east edge.
            GridPoint3::new(pos.root.next_west(), pos.y - half_y, end_y, pos.z)
        } else {
            // `pos` is either on an edge owned by its root,
            // or somewhere in the middle of the root.
            pos
        };

        PosInOwningRoot { pos: pos_in_owning_root }
    }

    /// Set z-coordinate of underlying `Pos`.
    ///
    /// Note that this is the one safe axis to operate
    /// on without knowing the globe resolution.
    pub fn set_z(&mut self, new_z: GridCoord) {
        self.pos.z = new_z;
    }
}

impl<'a> PosInOwningRoot {
    pub fn pos(&'a self) -> &'a GridPoint3 {
        &self.pos
    }
}

// Evil tricks to allow access to GridPoint2 fields from `self.rxy`
// as if they belong to `Self`.
impl Deref for GridPoint3 {
    type Target = GridPoint2;

    fn deref(&self) -> &GridPoint2 {
        &self.rxy
    }
}

impl DerefMut for GridPoint3 {
    fn deref_mut(&mut self) -> &mut GridPoint2 {
        &mut self.rxy
    }
}

/// Compare by root, z, y, then x.
///
/// Sorting points by this will be cache-friendly when indexing into,
/// e.g., `Chunk`s, which order their elements by z (coarsest) to x (finest).
pub fn semi_arbitrary_compare(a: &GridPoint3, b: &GridPoint3) -> Ordering {
    let cmp_root = a.root.index.cmp(&b.root.index);
    if cmp_root != Ordering::Equal {
        return cmp_root;
    }
    let cmp_z = a.z.cmp(&b.z);
    if cmp_z != Ordering::Equal {
        return cmp_z;
    }
    let cmp_y = a.y.cmp(&b.y);
    if cmp_y != Ordering::Equal {
        return cmp_y;
    }
    a.x.cmp(&b.x)
}
