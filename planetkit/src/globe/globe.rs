use std::collections::HashMap;

use specs;

use grid::{GridPoint3, PosInOwningRoot, Neighbors};
use super::{origin_of_chunk_owning, origin_of_chunk_in_same_root_containing};
use super::ChunkOrigin;
use super::chunk::{Chunk, Cell};
use super::spec::Spec;
use super::gen::Gen;
use super::chunk_pair::{ChunkPairOrigins, ChunkPair};

// TODO: split out a WorldGen type that handles all the procedural
// generation, because none of that really needs to be tangled
// with the realised Globe.
pub struct Globe {
    spec: Spec,
    // TODO: temporarily making this public because I'm planning to
    // rip it out of `Globe` anyway.
    pub gen: Gen,
    // Map chunk origins to chunks.
    //
    // TODO: you'll probably also want to store some lower-res
    // pseudo-chunks for rendering planets at a distance.
    // But maybe you can put that off? Or maybe that's an entirely
    // different type of Globe-oid?
    chunks: HashMap<ChunkOrigin, Chunk>,
    // Track which chunks are up-to-date with authoritative data for cells
    // they share with a neighbor.
    chunk_pairs: HashMap<ChunkPairOrigins, ChunkPair>,
}

// Allowing sibling modules to reach into semi-private parts
// of the Globe struct.
pub trait GlobeGuts<'a> {
    fn chunks(&'a self) -> &'a HashMap<ChunkOrigin, Chunk>;
    fn chunks_mut(&'a mut self) -> &'a mut HashMap<ChunkOrigin, Chunk>;
}

impl<'a> GlobeGuts<'a> for Globe {
    fn chunks(&'a self) -> &'a HashMap<ChunkOrigin, Chunk> {
        &self.chunks
    }

    fn chunks_mut(&'a mut self) -> &'a mut HashMap<ChunkOrigin, Chunk> {
        &mut self.chunks
    }
}

impl Globe {
    pub fn new(spec: Spec) -> Globe {
        Globe {
            spec: spec,
            gen: Gen::new(spec),
            chunks: HashMap::new(),
            chunk_pairs: HashMap::new(),
        }
    }

    pub fn new_example() -> Globe {
        Globe::new(Spec {
            seed: 14,
            floor_radius: 25.0,
            ocean_radius: 66.6,
            block_height: 0.65,
            root_resolution: [64, 128],
            // Chunks should probably be taller, but short chunks are a bit
            // better for now in exposing bugs visually.
            chunk_resolution: [16, 16, 4],
        })
    }

    pub fn new_earth_scale_example() -> Globe {
        Globe::new(Spec::new_earth_scale_example())
    }

    pub fn spec(&self) -> Spec {
        self.spec
    }

    /// Copy shared cells owned by a chunk for any loaded downstream chunks
    /// that have an outdated copy.
    ///
    /// Panics if the given chunk is not loaded.
    pub fn push_shared_cells_for_chunk(&mut self, source_chunk_origin: ChunkOrigin) {
        // BEWARE: HACKS BELOW to get around borrowck. There has to be
        // a better way around this!

        // Temporarily remove the source chunk from the globe, so that we can simultaneously
        // read from it and write to a bunch of other chunks.
        //
        // TODO: at very least avoid this most of the time by doing a read-only pass over
        // all neighbours and bailing out if they're completely up-to-date.
        let source_chunk = self.chunks.remove(&source_chunk_origin).expect(
            "Tried to push shared cells for a chunk that isn't loaded.",
        );

        // For each of this chunk's downstream neighbors, see if it has up-to-date
        // copies of the data we share with that neighbor. Otherwise, copy it over.
        for downstream_neighbor in &source_chunk.downstream_neighbors {
            let sink_chunk = match self.chunks.get_mut(&downstream_neighbor.origin) {
                None => {
                    // No worries; the chunk isn't loaded. Do nothing.
                    continue;
                }
                Some(chunk) => chunk,
            };

            // Look it up to see if it is already up-to-date.
            let chunk_pair_origins = ChunkPairOrigins {
                source: source_chunk.origin,
                sink: sink_chunk.origin,
            };
            let chunk_pair = self.chunk_pairs.get_mut(&chunk_pair_origins).expect(
                "Chunk pair for chunk should have been present.",
            );
            if chunk_pair.last_upstream_edge_version_known_downstream ==
                source_chunk.owned_edge_version
            {
                // Sink chunk is already up-to-date; move on to next neighbor.
                continue;
            }

            // Copy over each cell, one-by-one.
            for point_pair in &chunk_pair.point_pairs {
                let source_cell = *source_chunk.cell(point_pair.source.into());
                let target_cell = sink_chunk.cell_mut(point_pair.sink);
                // Copy source -> target.
                *target_cell = source_cell;
            }

            // Downstream chunk now has most recent changes from upstream.
            chunk_pair.last_upstream_edge_version_known_downstream = source_chunk
                .owned_edge_version;

            // If we got this far, then it means we needed to update something.
            // So mark the downstream chunk as having its view out-of-date.
            //
            // TODO: This isn't good enough; we sometimes decide
            // not to render cells (or parts thereof) on the edge
            // of a chunk based on what's next to them _vertically_,
            // so it's not enough to mark the sink chunk as dirty;
            // this needs to be a separate concept.
            //
            // Oh... there is totally a thing for that.
            // It's done in `remove_block` at the moment.
            // Sigh... we really need to clarify who's responsible
            // for what operations in this globe thing,
            // what what consistutes "edge cell" etc.
            sink_chunk.mark_view_as_dirty();
        }

        // Put the source chunk back into the world!
        self.chunks.insert(source_chunk_origin, source_chunk);
    }

    /// Copy shared cells not owned by a chunk from any loaded upstream chunks
    /// that have a more current copy.
    ///
    /// Panics if the given chunk is not loaded.
    pub fn pull_shared_cells_for_chunk(&mut self, sink_chunk_origin: ChunkOrigin) {
        // BEWARE: HACKS BELOW to get around borrowck. There has to be
        // a better way around this!

        // Temporarily remove the sink chunk from the globe, so that we can simultaneously
        // write to it and read from a bunch of other chunks.
        //
        // TODO: at very least avoid this most of the time by doing a read-only pass over
        // all neighbours and bailing out if we're completely up-to-date.
        let mut sink_chunk = self.chunks.remove(&sink_chunk_origin).expect(
            "Tried to pull shared cells for a chunk that isn't loaded.",
        );

        // Temporarily remove list of neighbours from sink chunk so that we can
        // both read from it and update the chunk's data.
        let mut upstream_neighbors = Vec::<super::chunk::UpstreamNeighbor>::new();
        use std::mem::swap;
        swap(&mut upstream_neighbors, &mut sink_chunk.upstream_neighbors);

        // For each of this chunk's upstream neighbors, see if it has a newer copy of the data
        // we share with that neighbor. Otherwise, copy it into the sink chunk.
        let mut sink_chunk_view_dirty = false;
        for upstream_neighbor in &upstream_neighbors {
            let source_chunk = match self.chunks.get_mut(&upstream_neighbor.origin) {
                None => {
                    // No worries; the chunk isn't loaded. Do nothing.
                    continue;
                }
                Some(chunk) => chunk,
            };

            // Look it up to see if it has any newer data.
            let chunk_pair_origins = ChunkPairOrigins {
                source: source_chunk.origin,
                sink: sink_chunk.origin,
            };
            let chunk_pair = self.chunk_pairs.get_mut(&chunk_pair_origins).expect(
                "Chunk pair for chunk should have been present.",
            );
            if chunk_pair.last_upstream_edge_version_known_downstream ==
                source_chunk.owned_edge_version
            {
                // Sink chunk is already up-to-date; move on to next neighbor.
                continue;
            }

            // Copy over each cell, one-by-one.
            for point_pair in &chunk_pair.point_pairs {
                let source_cell = *source_chunk.cell(point_pair.source.into());
                let target_cell = sink_chunk.cell_mut(point_pair.sink);
                // Copy source -> target.
                *target_cell = source_cell;
            }

            // Downstream chunk now has most recent changes from upstream.
            chunk_pair.last_upstream_edge_version_known_downstream = source_chunk
                .owned_edge_version;

            // If we got this far, then it means we needed to update something.
            sink_chunk_view_dirty = true;
        }

        // If we actually updated anything, then mark the downstream chunk
        // as having its view out-of-date.
        if sink_chunk_view_dirty {
            sink_chunk.mark_view_as_dirty();
        }

        // Put the list of neighbors back into the sink chunk.
        swap(&mut upstream_neighbors, &mut sink_chunk.upstream_neighbors);

        // Put the sink chunk back into the world!
        self.chunks.insert(sink_chunk_origin, sink_chunk);
    }

    pub fn origin_of_chunk_owning(&self, pos: PosInOwningRoot) -> ChunkOrigin {
        origin_of_chunk_owning(pos, self.spec.root_resolution, self.spec.chunk_resolution)
    }

    // NOTE: chunk returned probably won't _own_ `pos`.
    pub fn origin_of_chunk_in_same_root_containing(&self, pos: GridPoint3) -> ChunkOrigin {
        // Figure out what chunk this is in.
        origin_of_chunk_in_same_root_containing(
            pos,
            self.spec.root_resolution,
            self.spec.chunk_resolution,
        )
    }

    /// Most `Chunks`s will have an associated `ChunkView`. Indicate that the
    /// chunk (or something else affecting its visibility) has been modified
    /// since the view was last updated.
    pub fn mark_chunk_views_affected_by_cell_as_dirty(&mut self, pos_in_owning_root: PosInOwningRoot) {
        // TODO: really, just rewrite this whole function. It doesn't really work.

        // TODO: this (Vec) is super slow! Replace with a less brain-dead solution.
        // Just committing this one now to patch over a kinda-regression in that
        // the existing bug of not doing this at all just become a lot more
        // obvious now that I'm doing a better job of culling cells.
        let mut dirty_cells: Vec<PosInOwningRoot> = Vec::new();
        dirty_cells.push(pos_in_owning_root);
        dirty_cells.extend(
            Neighbors::new(pos_in_owning_root.into(), self.spec.root_resolution)
                .map(|neighbor_pos| {
                    PosInOwningRoot::new(neighbor_pos, self.spec.root_resolution)
                }),
        );
        // Gah, filthy hacks. This is to get around not having a way to query for
        // "all chunks containing this cell".
        //
        // TODO: replace now that you DO have that. See other comment about `AllChunksContainingPoint`.
        let mut cells_in_dirty_chunks: Vec<PosInOwningRoot> = Vec::new();
        for dirty_cell in dirty_cells {
            cells_in_dirty_chunks.extend(
                Neighbors::new(dirty_cell.into(), self.spec.root_resolution)
                    .map(|neighbor_pos| {
                        PosInOwningRoot::new(neighbor_pos, self.spec.root_resolution)
                    }),
            );
        }
        for dirty_pos in cells_in_dirty_chunks {
            let chunk_origin = self.origin_of_chunk_owning(dirty_pos);
            // It's fine for the chunk to not be loaded.
            if let Some(chunk) = self.chunks.get_mut(&chunk_origin) {
                chunk.mark_view_as_dirty();
            }
        }
    }

    pub fn increment_chunk_owned_edge_version_for_cell(&mut self, pos: PosInOwningRoot) {
        let chunk_origin = self.origin_of_chunk_owning(pos.into());
        let chunk = self.chunks.get_mut(&chunk_origin).expect(
            "Uh oh, I don't know how to handle chunks that aren't loaded yet.",
        );
        chunk.owned_edge_version += 1;
    }

    /// Add the given chunk to the globe.
    ///
    /// This may have been freshly generated, or loaded from disk.
    ///
    /// # Panics
    ///
    /// Panics if there was already a chunk loaded for the same chunk origin.
    pub fn add_chunk(&mut self, chunk: Chunk) {
        // TODO: You could assert that the chunk actually belongs to _this globe_.
        // It could store a UUID associated with its generator and complain
        // if you try to load a chunk for the wrong globe even if they have
        // the same resolution.

        self.ensure_all_chunk_pairs_present_for(&chunk);

        let chunk_origin = chunk.origin;
        if self.chunks.insert(chunk_origin, chunk).is_some() {
            panic!("There was already a chunk loaded at the same origin!");
        }
    }

    /// Remove the chunk at the given chunk origin. Returns the removed chunk.
    ///
    /// # Panics
    ///
    /// Panics if there was no chunk loaded at the given chunk origin.
    pub fn remove_chunk(&mut self, chunk_origin: ChunkOrigin) -> Chunk {
        let chunk = self.chunks.remove(&chunk_origin).expect(
            "Attempted to remove a chunk that was not loaded",
        );
        self.remove_all_chunk_pairs_for(&chunk);
        chunk
    }

    // TODO: consider moving `load_or_build_chunk`, `ensure_chunk_present`,
    // and `find_lowest_cell_containing` back out into a smarter component
    // so that `Globe` can be dumber, or move more of `Globe` down into a new
    // dumber component, e.g., `GlobeVoxMap`.

    pub fn load_or_build_chunk(&mut self, origin: ChunkOrigin) {
        use rand;
        use rand::Rng;

        let spec = self.spec();

        let mut cells: Vec<Cell> = Vec::new();
        // Include cells _on_ the far edge of the chunk;
        // even though we don't own them we'll need to draw part of them.
        let end_x = origin.pos().x + spec.chunk_resolution[0];
        let end_y = origin.pos().y + spec.chunk_resolution[1];
        // Chunks don't share cells in the z-direction,
        // but do in the x- and y-directions.
        let end_z = origin.pos().z + spec.chunk_resolution[2] - 1;
        for cell_z in origin.pos().z..(end_z + 1) {
            for cell_y in origin.pos().y..(end_y + 1) {
                for cell_x in origin.pos().x..(end_x + 1) {
                    let grid_point = GridPoint3::new(origin.pos().root, cell_x, cell_y, cell_z);
                    let mut cell = self.gen.cell_at(grid_point);
                    // Temp hax?
                    let mut rng = rand::thread_rng();
                    cell.shade = 1.0 - 0.5 * rng.next_f32();
                    cells.push(cell);
                }
            }
        }
        self.add_chunk(Chunk::new(
            origin,
            cells,
            spec.root_resolution,
            spec.chunk_resolution,
        ));
    }

    /// Ensures the specified chunk is present.
    ///
    /// If the chunk is already present, then do nothing. Otherwise, the chunk
    /// may be either loaded from disk, or generated fresh if it has never been
    /// saved.
    ///
    /// This pays no regard to preferred limits on the number of chunks that should
    /// be loaded, and chunks added through this mechanism may well be unloaded
    /// immediately the next time this system is invoked, making this only suitable
    /// for immediate actions.
    //
    // TODO: this definitely belongs elsewhere; somewhere that knows about
    // loading things from disk. The simpler version of just making sure the
    // voxmap buffer exists should exist on a dumber struct extracted from `Globe`.
    pub fn ensure_chunk_present(&mut self, chunk_origin: ChunkOrigin) {
        if self.chunk_at(chunk_origin).is_some() {
            return;
        }
        self.load_or_build_chunk(chunk_origin);

        // Make sure this chunk has up-to-date data for edge cells that it doesn't own.
        self.pull_shared_cells_for_chunk(chunk_origin);

        // Make sure that neighboring chunks have up-to-date data for edge cells owned
        // by this chunk.
        self.push_shared_cells_for_chunk(chunk_origin);
    }

    /// Make sure we are tracking the currency of shared data in all chunks
    /// upstream or downstream of this chunk.
    fn ensure_all_chunk_pairs_present_for(&mut self, chunk: &Chunk) {
        use globe::chunk_pair::{ChunkPairOrigins, ChunkPair};
        // Copy cached upstream and downstream neighbors from
        // chunks rather than re-computing them for each pair now.
        for upstream_neighbor in &chunk.upstream_neighbors {
            let chunk_pair_origins = ChunkPairOrigins {
                source: upstream_neighbor.origin,
                sink: chunk.origin,
            };
            self.chunk_pairs.entry(chunk_pair_origins).or_insert(
                ChunkPair {
                    point_pairs: upstream_neighbor.shared_cells.clone(),
                    last_upstream_edge_version_known_downstream: 0,
                },
            );
        }
        for downstream_neighbor in &chunk.downstream_neighbors {
            let chunk_pair_origins = ChunkPairOrigins {
                source: chunk.origin,
                sink: downstream_neighbor.origin,
            };
            self.chunk_pairs.entry(chunk_pair_origins).or_insert(
                ChunkPair {
                    point_pairs: downstream_neighbor.shared_cells.clone(),
                    last_upstream_edge_version_known_downstream: 0,
                },
            );
        }
    }

    /// Clean up any chunk pairs where either the source or sink is this chunk.
    fn remove_all_chunk_pairs_for(&mut self, chunk: &Chunk) {
        // TODO: HashMap::retain was stabilised in Rust 1.18;
        // replace this as soon as you update.
        for upstream_neighbor in &chunk.upstream_neighbors {
            let chunk_pair_origins = ChunkPairOrigins {
                source: upstream_neighbor.origin,
                sink: chunk.origin,
            };
            // Note that chunk pair will only be present if _both_ chunks it refers to were loaded.
            self.chunk_pairs.remove(&chunk_pair_origins);
        }
        for downstream_neighbor in &chunk.downstream_neighbors {
            let chunk_pair_origins = ChunkPairOrigins {
                source: chunk.origin,
                sink: downstream_neighbor.origin,
            };
            // Note that chunk pair will only be present if _both_ chunks it refers to were loaded.
            self.chunk_pairs.remove(&chunk_pair_origins);
        }
    }
}

impl<'a> Globe {
    pub fn chunk_at(&'a self, chunk_origin: ChunkOrigin) -> Option<&'a Chunk> {
        self.chunks.get(&chunk_origin)
    }

    // TODO: this naming is horrible. :)

    pub fn authoritative_cell(&'a self, pos: PosInOwningRoot) -> &'a Cell {
        let chunk_origin = self.origin_of_chunk_owning(pos);
        let chunk = self.chunks.get(&chunk_origin).expect(
            "Uh oh, I don't know how to handle chunks that aren't loaded yet.",
        );
        chunk.cell(pos.into())
    }

    pub fn authoritative_cell_mut(&'a mut self, pos: PosInOwningRoot) -> &'a mut Cell {
        let chunk_origin = self.origin_of_chunk_owning(pos);
        let chunk = self.chunks.get_mut(&chunk_origin).expect(
            "Uh oh, I don't know how to handle chunks that aren't loaded yet.",
        );
        chunk.cell_mut(pos.into())
    }

    // TODO: proper error type
    // Panic message formerly "Uh oh, I don't know how to handle chunks that aren't loaded yet."
    pub fn maybe_non_authoritative_cell(&'a self, pos: GridPoint3) -> Result<&'a Cell, ()> {
        let chunk_origin = self.origin_of_chunk_in_same_root_containing(pos);
        self.chunks.get(&chunk_origin).map(|chunk| {
            chunk.cell(pos)
        }).ok_or(())
    }
}

impl specs::Component for Globe {
    type Storage = specs::HashMapStorage<Globe>;
}
