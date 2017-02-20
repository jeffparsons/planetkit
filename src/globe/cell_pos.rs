use globe::{ IntCoord, Root, RootIndex };

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct CellPos {
    pub root: Root,
    pub x: IntCoord,
    pub y: IntCoord,
    pub z: IntCoord,
}

impl CellPos {
    // Convenience methods, mostly for testing.
    // This is kind of like the builder pattern
    // in that it allows chaining by returning self.
    //
    // I toyed with using a proper builder for this
    // but its use was just too verbose to justify.
    //
    // TODO: move them into a special module that you
    // only import in tests?

    // TODO: get rid of these; they are too easy to misuse.

    pub fn set_root(mut self, new_root_index: RootIndex) -> Self {
        self.root.index = new_root_index;
        self
    }

    pub fn set_x(mut self, new_x: IntCoord) -> Self {
        self.x = new_x;
        self
    }

    pub fn set_y(mut self, new_y: IntCoord) -> Self {
        self.y = new_y;
        self
    }

    pub fn set_z(mut self, new_z: IntCoord) -> Self {
        self.z = new_z;
        self
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
            CellPos {
                // First root owns north pole.
                root: 0.into(),
                x: 0,
                y: 0,
                z: pos.z,
            }
        } else if pos.x == end_x && pos.y == end_y {
            // South pole
            CellPos {
                // Last root owns south pole.
                root: 4.into(),
                x: end_x,
                y: end_y,
                z: pos.z,
            }
        } else if pos.y == 0 {
            // Roots don't own their north-west edge;
            // translate to next root's north-east edge.
            CellPos {
                root: pos.root.next_west(),
                x: 0,
                y: pos.x,
                z: pos.z,
            }
        } else if pos.x == end_x && pos.y < half_y {
            // Roots don't own their mid-west edge;
            // translate to the next root's mid-east edge.
            CellPos {
                root: pos.root.next_west(),
                x: 0,
                y: half_y + pos.y,
                z: pos.z,
            }
        } else if pos.x == end_x {
            // Roots don't own their south-west edge;
            // translate to the next root's south-east edge.
            CellPos {
                root: pos.root.next_west(),
                y: end_y,
                x: pos.y - half_y,
                z: pos.z,
            }
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

/// Wrapper type around a `Pos` that is known to express
/// a valid chunk origin.
///
/// Note that this does not save you from accidentally using
/// positions from multiple incompatible `Globe`s with different
/// resolutions.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ChunkOrigin {
    pos: CellPos,
}

impl Into<CellPos> for ChunkOrigin {
    fn into(self) -> CellPos {
        self.pos
    }
}

impl ChunkOrigin {
    // Asserts that `pos` is a valid chunk origin at the given `resolution`,
    // and returns a `ChunkOrigin` wrapping it.
    //
    // # Panics
    //
    // Panics if `pos` is not a valid chunk origin.
    pub fn new(pos: CellPos, root_resolution: [IntCoord; 2], chunk_resolution: [IntCoord; 3]) -> ChunkOrigin {
        // Make sure `pos` is within bounds.
        assert!(pos.x >= 0);
        assert!(pos.y >= 0);
        assert!(pos.z >= 0);
        assert!(pos.x < root_resolution[0]);
        assert!(pos.y < root_resolution[1]);

        // Chunk origins sit at multiples of `chunk_resolution[axis_index]`.
        assert!(pos.x == pos.x / chunk_resolution[0] * chunk_resolution[0]);
        assert!(pos.y == pos.y / chunk_resolution[1] * chunk_resolution[1]);
        assert!(pos.z == pos.z / chunk_resolution[2] * chunk_resolution[2]);

        ChunkOrigin {
            pos: pos,
        }
    }
}

// TODO: Should this actually be an implementation of Deref? Try it...
impl<'a> ChunkOrigin {
    pub fn pos(&'a self) -> &'a CellPos {
        &self.pos
    }
}
