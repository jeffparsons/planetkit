use std::collections::HashMap;

use specs;
use globe::{ IntCoord, CellPos, ChunkOrigin, PosInOwningRoot };
use globe::origin_of_chunk_owning;

#[derive(PartialEq, Eq, Copy, Clone)]
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
pub struct Cell {
    pub material: Material,
    pub shade: f32,
}

// Stores from (0, 0) to (chunk_resolution, chunk_resolution) _inclusive_.
//
// TODO: copy storage layout and cell ownership rules from:
// <http://kiwi.atmos.colostate.edu/BUGS/geodesic/text.html>.
// They seem to have a pretty good grasp on these things. :)
pub struct Chunk {
    pub origin: ChunkOrigin,
    pub chunk_resolution: [IntCoord; 3],
    // Sorted by (z, y, x).
    pub cells: Vec<Cell>,
    pub view_entity: Option<specs::Entity>,
    // Incremented whenever authoritative data in this chunk is updated.
    // This way even if a neighboring chunk was not loaded when we update this chunk,
    // we can detect later that it is out-of-date.
    //
    // The first version is 1, so we can use 0 to represent "no last known version"
    // of our neighboring chunks.
    pub version: u64,
    // Neighbors that are the source of truth for some of the cells on
    // the border of this chunk.
    pub authoritative_neighbors: Vec<Neighbor>,
    pub is_view_dirty: bool,
}

impl Chunk {
    pub fn new(
        origin: ChunkOrigin,
        cells: Vec<Cell>,
        root_resolution: [IntCoord; 2],
        chunk_resolution: [IntCoord; 3],
    ) -> Chunk {
        Chunk {
            origin: origin,
            cells: cells,
            chunk_resolution: chunk_resolution,
            view_entity: None,
            version: 1,
            authoritative_neighbors: Self::list_authoritative_neighbors(
                origin,
                root_resolution,
                chunk_resolution
            ),
            is_view_dirty: true,
        }
    }

    fn list_authoritative_neighbors(
        origin: ChunkOrigin,
        root_resolution: [IntCoord; 2],
        chunk_resolution: [IntCoord; 3],
    ) -> Vec<Neighbor> {
        // Map neighbor chunk origin to neighbors for efficient lookup
        // during construction.
        let mut neighbors_by_origin = HashMap::<ChunkOrigin, Neighbor>::new();

        // For every cell, if its owning chunk is not this chunk,
        // then add it to the list that we might need to copy from.
        let end_x = origin.pos().x + chunk_resolution[0];
        let end_y = origin.pos().y + chunk_resolution[1];
        // Chunks don't share cells in the z-direction,
        // but do in the x- and y-directions.
        let end_z = origin.pos().z + chunk_resolution[2] - 1;
        // Iterating over _all_ cells is a dumb slow way to do this,
        // but we don't do it very often. So... meh. :)
        for cell_z in origin.pos().z..(end_z + 1) {
            for cell_y in origin.pos().y..(end_y + 1) {
                for cell_x in origin.pos().x..(end_x + 1) {
                    let other_pos = CellPos {
                        root: origin.pos().root,
                        x: cell_x,
                        y: cell_y,
                        z: cell_z,
                    };

                    // Find what chunk this belongs to.
                    let other_pos_in_owning_root = PosInOwningRoot::new(
                        other_pos, root_resolution
                    );
                    let other_pos_chunk_origin = origin_of_chunk_owning(other_pos_in_owning_root, root_resolution, chunk_resolution);
                    if other_pos_chunk_origin == origin {
                        // We own this cell; nothing to do.
                        continue;
                    }

                    // We don't own this cell; ensure there's a record for the neighboring
                    // chunk that it belongs to, and add it to the list of relevant cells.
                    let mut neighbor = neighbors_by_origin
                        .entry(other_pos_chunk_origin)
                        .or_insert(Neighbor {
                            origin: other_pos_chunk_origin,
                            // We've never pulled from this neighbour.
                            last_known_version: 0,
                            shared_cells: Vec::new(),
                        });
                    neighbor.shared_cells.push(other_pos);
                }
            }
        }

        // We'll usually just want to iterate over these. No need to store
        // as a hash map beyond building it.
        neighbors_by_origin.values().cloned().collect()
    }

    // Panics or returns nonsense if given coordinates of a cell we don't have data for.
    //
    // TODO: _store_ more information to make lookups cheaper.
    fn cell_index(&self, pos: CellPos) -> usize {
        let local_x = pos.x - self.origin.pos().x;
        let local_y = pos.y - self.origin.pos().y;
        let local_z = pos.z - self.origin.pos().z;
        (
            local_z * (self.chunk_resolution[0] + 1) * (self.chunk_resolution[1] + 1) +
            local_y * (self.chunk_resolution[0] + 1) +
            local_x
        ) as usize
    }

    /// Returns `true` if the given `pos` lies within the bounds of this chunk,
    /// or `false` otherwise.
    ///
    /// Note that this does not consider whether or not this chunk _owns_ the
    /// cell at `pos`.
    pub fn contains_pos(&self, pos: CellPos) -> bool {
        // Chunks don't share cells in the z-direction,
        // but do in the x- and y-directions.
        let end_x = self.origin.pos().x + self.chunk_resolution[0];
        let end_y = self.origin.pos().y + self.chunk_resolution[1];
        let end_z = self.origin.pos().z + self.chunk_resolution[2] - 1;
        pos.x >= self.origin.pos().x && pos.x <= end_x &&
        pos.y >= self.origin.pos().y && pos.y <= end_y &&
        pos.z >= self.origin.pos().z && pos.z <= end_z
    }

    /// Most `Chunks`s will have an associated `ChunkView`. Indicate that the
    /// chunk has been modified since the view was last updated.
    pub fn mark_view_as_dirty(&mut self) {
        self.is_view_dirty = true;
    }

    /// Most `Chunks`s will have an associated `ChunkView`. Indicate that the
    /// view has been updated since the chunk was last modified.
    pub fn mark_view_as_clean(&mut self) {
        self.is_view_dirty = false;
    }
}

impl<'a> Chunk {
    // TODO: replace these with two variants:
    // - one that clearly is asking for authoritative data,
    //   and requires a PosInOwningChunk (does not yet exist)
    // - one that is happy to get any old version of the cell.

    // Panics if given coordinates of a cell we don't have data for.
    pub fn cell(&'a self, pos: CellPos) -> &'a Cell {
        let cell_i = self.cell_index(pos);
        &self.cells[cell_i]
    }

    // Panics if given coordinates of a cell we don't have data for.
    pub fn cell_mut(&'a mut self, pos: CellPos) -> &'a mut Cell {
        let cell_i = self.cell_index(pos);
        &mut self.cells[cell_i]
    }
}

#[derive(Clone)]
pub struct Neighbor {
    // TODO: visibility

    pub origin: ChunkOrigin,
    pub last_known_version: u64,
    // TODO: make this a "trusted to be in owner" wrapper.
    pub shared_cells: Vec<CellPos>,
}
