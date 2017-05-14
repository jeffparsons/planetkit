use grid::{ GridCoord, GridPoint3 };

/// Wrapper type around a `Pos` that is known to express
/// a valid chunk origin.
///
/// Note that this does not save you from accidentally using
/// positions from multiple incompatible `Globe`s with different
/// resolutions.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ChunkOrigin {
    pos: GridPoint3,
}

impl Into<GridPoint3> for ChunkOrigin {
    fn into(self) -> GridPoint3 {
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
    pub fn new(pos: GridPoint3, root_resolution: [GridCoord; 2], chunk_resolution: [GridCoord; 3]) -> ChunkOrigin {
        // Make sure `pos` is within bounds.
        assert!(pos.x >= 0);
        assert!(pos.y >= 0);
        assert!(pos.z >= 0);
        assert!(pos.x < root_resolution[0]);
        assert!(pos.y < root_resolution[1]);

        // Chunk origins sit at multiples of `chunk_resolution[axis_index]`.
        assert_eq!(pos.x, pos.x / chunk_resolution[0] * chunk_resolution[0]);
        assert_eq!(pos.y, pos.y / chunk_resolution[1] * chunk_resolution[1]);
        assert_eq!(pos.z, pos.z / chunk_resolution[2] * chunk_resolution[2]);

        ChunkOrigin {
            pos: pos,
        }
    }
}

// TODO: Should this actually be an implementation of Deref? Try it...
impl<'a> ChunkOrigin {
    pub fn pos(&'a self) -> &'a GridPoint3 {
        &self.pos
    }
}
