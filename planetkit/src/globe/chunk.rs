use specs;
use grid::{GridCoord, GridPoint3, PosInOwningRoot};
use globe::ChunkOrigin;
use globe::origin_of_chunk_owning;
use globe::chunk_pair::PointPair;

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
// Storage layout and cell ownership rules are mostly following:
// <http://kiwi.atmos.colostate.edu/BUGS/geodesic/text.html>.
// They seem to have a pretty good grasp on these things. :)
pub struct Chunk {
    pub origin: ChunkOrigin,
    pub chunk_resolution: [GridCoord; 3],
    // Sorted by (z, y, x).
    pub cells: Vec<Cell>,
    pub view_entity: Option<specs::Entity>,
    // Incremented whenever authoritative data in this chunk that is shared with other chunks is updated.
    // This way even if a neighboring chunk was not loaded when we update this chunk,
    // we can detect later that it is out-of-date.
    //
    // The first version is 1, so we can use 0 to represent "no last known version"
    // of our neighboring chunks.
    pub owned_edge_version: u64,
    // Neighbors that are the source of truth for some of the cells on the border of this chunk.
    //
    // TODO: when we switch to froggy storage, wrap both of these in a struct that
    // stores a weak pointer to where we think the chunk might be loaded,
    // so that we don't need to look it up in a hash map.
    pub upstream_neighbors: Vec<UpstreamNeighbor>,
    // Neighbors that share some cells on the border of this chunk but don't own them.
    pub downstream_neighbors: Vec<DownstreamNeighbor>,
    pub is_view_dirty: bool,
    // Chunks that are directly accessible from the given chunk via a single,
    // step between cells, including this chunk itself.
    //
    // Note that this might not be adequate to find all chunks accessible via a single
    // user action, e.g., stepping a `CellDweller`, because that action might lead
    // to movement across multiple cells -- in this case, stepping up a block moves
    // one space forward, and one up, so take care if using this to ensure the relevant
    // chunks are loaded.
    //
    // TODO: unlike `authoritative_neighbors`, which tracks version numbers,
    // this doesn't really belong here. I'm just storing it here for now because
    // it's inefficient to compute and most of the time when you'll want it, you'll already
    // have the chunk loaded.
    //
    // TODO: look at clients, and consider whether a separate lazily-cached
    // map of these on Globe might be more appropriate.
    pub accessible_chunks: Vec<ChunkOrigin>,
}

impl Chunk {
    pub fn new(
        origin: ChunkOrigin,
        cells: Vec<Cell>,
        root_resolution: [GridCoord; 2],
        chunk_resolution: [GridCoord; 3],
    ) -> Chunk {
        let mut chunk = Chunk {
            origin: origin,
            cells: cells,
            chunk_resolution: chunk_resolution,
            view_entity: None,
            owned_edge_version: 1,
            upstream_neighbors: Vec::new(),
            downstream_neighbors: Vec::new(),
            is_view_dirty: true,
            accessible_chunks: Self::list_accessible_chunks(
                origin,
                root_resolution,
                chunk_resolution,
            ),
        };
        chunk.populate_neighboring_chunks(root_resolution);
        chunk
    }

    /// Find and store the origins of all upstream and downstream neighbor chunks.
    ///
    /// Panics if called more than once; it is for initialization only.
    fn populate_neighboring_chunks(&mut self, root_resolution: [GridCoord; 2]) {
        use grid::EquivalentPoints;
        use grid::semi_arbitrary_compare;
        use globe::ChunksInSameRootContainingPoint;

        if self.upstream_neighbors.len() > 0 || self.downstream_neighbors.len() > 0 {
            panic!("Tried to initialize chunk multiple times.");
        }

        // Map neighbor chunk origins to neighbors for easy lookup during construction.
        use std::collections::HashMap;
        use globe::ChunkSharedPoints;
        let mut upstream_neighbors_by_origin = HashMap::<ChunkOrigin, UpstreamNeighbor>::new();
        let mut downstream_neighbors_by_origin = HashMap::<ChunkOrigin, DownstreamNeighbor>::new();

        // For every point on the edge of this chunk, find all the chunks that contain it,
        // taking note that some of those might be in other roots if the point is on the edge of a root.
        // Then, depending on whether or not we own the point, add it to a downstream neighbor
        // or an upstream neighbor.
        let shared_points = ChunkSharedPoints::new(self.origin, self.chunk_resolution);
        for our_point in shared_points {
            // Find what chunk this belongs to.
            let our_point_in_owning_root = PosInOwningRoot::new(our_point, root_resolution);
            let owning_chunk_origin = origin_of_chunk_owning(
                our_point_in_owning_root,
                root_resolution,
                self.chunk_resolution,
            );
            let we_own_this_point = owning_chunk_origin == self.origin;

            // TODO: wrap this up as an `AllChunksContainingPoint` iterator.
            // TODO: this is perfect as an "easy"-tagged github issue. Maybe try carving some of these off.
            let equivalent_points = EquivalentPoints::new(our_point, root_resolution);
            for equivalent_point in equivalent_points {
                let containing_chunks = ChunksInSameRootContainingPoint::new(
                    equivalent_point,
                    root_resolution,
                    self.chunk_resolution,
                );
                for chunk_origin in containing_chunks {
                    if chunk_origin == self.origin {
                        // We're looking at the same chunk; we'll never need to copy to/from self!
                        continue;
                    }

                    if we_own_this_point {
                        // We own the cell; ensure there's a record for the downstream chunk,
                        // and then add the pair of representations of the same point to the list
                        // of cells that need to be synced.
                        let downstream_neighbor = downstream_neighbors_by_origin
                            .entry(chunk_origin)
                            .or_insert(DownstreamNeighbor {
                                origin: chunk_origin,
                                shared_cells: Vec::new(),
                            });
                        downstream_neighbor.shared_cells.push(PointPair {
                            source: our_point_in_owning_root,
                            sink: equivalent_point,
                        });
                    } else if owning_chunk_origin == chunk_origin {
                        // The other chunk owns the cell; ensure there's a record for the upstream chunk,
                        // and then add the pair of representations of the same point to the list
                        // of cells that need to be synced.
                        let upstream_neighbor = upstream_neighbors_by_origin
                            .entry(chunk_origin)
                            .or_insert(UpstreamNeighbor {
                                origin: chunk_origin,
                                shared_cells: Vec::new(),
                            });
                        let equivalent_point_in_owning_root =
                            PosInOwningRoot::new(equivalent_point, root_resolution);
                        upstream_neighbor.shared_cells.push(PointPair {
                            source: equivalent_point_in_owning_root,
                            sink: our_point,
                        });
                    }
                }
            }
        }

        // We'll usually just want to iterate over these. No need to store
        // as a hash map beyond building it.
        //
        // Sort the points inside these for cache-friendliness.
        self.upstream_neighbors = upstream_neighbors_by_origin.values().cloned().collect();
        for upstream_neighbor in &mut self.upstream_neighbors {
            upstream_neighbor.shared_cells.sort_by(|point_pair_a,
             point_pair_b| {
                // Comparing by source points should give us some tiny cache locality benefit.
                // So would sorting by sink points, but hopefully better one sorted than neither.
                // TODO: check whether this helps at all.
                semi_arbitrary_compare(point_pair_a.source.pos(), &point_pair_b.source.pos())
            });
        }
        self.downstream_neighbors = downstream_neighbors_by_origin.values().cloned().collect();
        for downstream_neighbor in &mut self.downstream_neighbors {
            downstream_neighbor.shared_cells.sort_by(|point_pair_a,
             point_pair_b| {
                // Comparing by sink points should give us some tiny cache locality benefit.
                // So would sorting by source points, but hopefully better one sorted than neither.
                // TODO: check whether this helps at all.
                semi_arbitrary_compare(&point_pair_a.sink, &point_pair_b.sink)
            });
        }
    }

    // Panics or returns nonsense if given coordinates of a cell we don't have data for.
    //
    // TODO: _store_ more information to make lookups cheaper.
    fn cell_index(&self, pos: GridPoint3) -> usize {
        let local_x = pos.x - self.origin.pos().x;
        let local_y = pos.y - self.origin.pos().y;
        let local_z = pos.z - self.origin.pos().z;
        let r = self.chunk_resolution;
        let plane_offset = local_z * (r[0] + 1) * (r[1] + 1);
        let row_offset = local_y * (r[0] + 1);
        let cell_offset = local_x;
        (plane_offset + row_offset + cell_offset) as usize
    }

    /// Returns `true` if the given `pos` lies within the bounds of this chunk,
    /// or `false` otherwise.
    ///
    /// Note that this does not consider whether or not this chunk _owns_ the
    /// cell at `pos`.
    pub fn contains_pos(&self, pos: GridPoint3) -> bool {
        // Chunks don't share cells in the z-direction,
        // but do in the x- and y-directions.
        let end_x = self.origin.pos().x + self.chunk_resolution[0];
        let end_y = self.origin.pos().y + self.chunk_resolution[1];
        let end_z = self.origin.pos().z + self.chunk_resolution[2] - 1;
        let contains_x = pos.x >= self.origin.pos().x && pos.x <= end_x;
        let contains_y = pos.y >= self.origin.pos().y && pos.y <= end_y;
        let contains_z = pos.z >= self.origin.pos().z && pos.z <= end_z;
        contains_x && contains_y && contains_z
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

    fn list_accessible_chunks(
        origin: ChunkOrigin,
        root_resolution: [GridCoord; 2],
        chunk_resolution: [GridCoord; 3],
    ) -> Vec<ChunkOrigin> {
        // Keep track of which chunk origins we've seen.
        use std::collections::HashSet;
        let mut chunk_origins = HashSet::<ChunkOrigin>::new();

        // For every cell at a corner of this chunk, find all its neighbors,
        // and add the origins of their chunks.
        for (x, y, z) in iproduct!(
            &[origin.pos().x, origin.pos().x + chunk_resolution[0]],
            &[origin.pos().y, origin.pos().y + chunk_resolution[1]],
            // Chunks don't share cells in the z-direction,
            // but do in the x- and y-directions.
            &[origin.pos().z, origin.pos().z + chunk_resolution[2] - 1]
        )
        {
            let corner_pos = GridPoint3::new(origin.pos().root, *x, *y, *z);
            // Find all its neighbors and their chunks' origins.
            //
            // TODO: does Neighbors actually guarantee that we'll get chunks
            // from the roots we intend? I don't think so! Maybe we should introduce
            // some way to explicitly list all the equivalent representations of
            // a pos and use that instead here, because looking at "neighbors"
            // here is actually just a hack workaround for not having that.
            use grid::Neighbors;
            use super::origin_of_chunk_in_same_root_containing;
            let neighbors = Neighbors::new(corner_pos, root_resolution);
            for neighbor in neighbors {
                let neighbor_chunk_origin = origin_of_chunk_in_same_root_containing(
                    neighbor,
                    root_resolution,
                    chunk_resolution,
                );
                chunk_origins.insert(neighbor_chunk_origin);
            }
        }

        // We'll usually just want to iterate over these. No need to store
        // as a hash map beyond building it.
        chunk_origins.iter().cloned().collect()
    }
}

impl<'a> Chunk {
    // TODO: replace these with two variants:
    // - one that clearly is asking for authoritative data,
    //   and requires a PosInOwningChunk (does not yet exist)
    // - one that is happy to get any old version of the cell.

    // Panics if given coordinates of a cell we don't have data for.
    pub fn cell(&'a self, pos: GridPoint3) -> &'a Cell {
        let cell_i = self.cell_index(pos);
        &self.cells[cell_i]
    }

    // Panics if given coordinates of a cell we don't have data for.
    pub fn cell_mut(&'a mut self, pos: GridPoint3) -> &'a mut Cell {
        let cell_i = self.cell_index(pos);
        &mut self.cells[cell_i]
    }
}

#[derive(Clone)]
pub struct UpstreamNeighbor {
    // TODO: visibility? pub(::globe)?
    pub origin: ChunkOrigin,
    // List of positions owned by the neighbor (source), specified in both source and sink.
    pub shared_cells: Vec<PointPair>,
}

#[derive(Clone)]
pub struct DownstreamNeighbor {
    // TODO: visibility? pub(::globe)?
    pub origin: ChunkOrigin,
    // List of positions owned by this chunk (source), specified in both source and sink.
    pub shared_cells: Vec<PointPair>,
}
