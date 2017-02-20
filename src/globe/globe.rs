use std::collections::HashMap;

use specs;

use slog::Logger;

use super::{ origin_of_chunk_owning, origin_of_chunk_in_same_root_containing };
use super::Root;
use super::{ CellPos, PosInOwningRoot, ChunkOrigin };
use super::Neighbors;
use super::chunk::{ Chunk, Cell, Material };
use super::spec::Spec;
use super::gen::Gen;
use ::Spatial;

// TODO: lift to module level.
const ROOT_QUADS: u8 = 5;

// TODO: how many to build high?
// TODO: remove me
const Z_CHUNKS: i64 = 5;

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
    // TODO: figure out what structure to store these in.
    // You'll never have all chunks loaded in the real world.
    //
    // TODO: you'll probably also want to store some lower-res
    // pseudo-chunks for rendering planets at a distance.
    // But maybe you can put that off?
    chunks: HashMap<ChunkOrigin, Chunk>,
    log: Logger,
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
    pub fn new(spec: Spec, parent_log: &Logger) -> Globe {
        let globe = Globe {
            spec: spec,
            gen: Gen::new(spec),
            chunks: HashMap::new(),
            log: parent_log.new(o!()),
        };
        globe
    }

    pub fn new_example(parent_log: &Logger) -> Globe {
        Globe::new(
            Spec {
                seed: 13,
                floor_radius: 0.91, // TODO: make it ~Earth
                // NOTE: Don't let ocean radius be a neat multiple of block
                // height above floor radius, or we'll end up with
                // z-fighting in evaluating what blocks are water/air.
                ocean_radius: 1.13,
                block_height: 0.02,
                root_resolution: [32, 64],
                chunk_resolution: [16, 16, 4],
                flat: false,
            },
            parent_log,
        )
    }

    pub fn new_small_flat(parent_log: &Logger) -> Globe {
        Globe::new(
            Spec {
                seed: 13,
                floor_radius: 0.91,
                ocean_radius: 1.13,
                block_height: 0.02,
                root_resolution: [8, 16],
                chunk_resolution: [4, 4, 4],
                flat: true,
            },
            parent_log,
        )
    }

    pub fn spec(&self) -> Spec {
        self.spec
    }

    // TODO: there's no way this should be public.
    // Replace with a better interface for mutating cell content
    // that automatically ensures that all neighbouring chunks
    // get updated, etc.
    pub fn copy_all_authoritative_cells(&mut self) {
        // Calculate how many chunks to a root in each direction in (x, y).
        let chunks_per_root = [
            self.spec.root_resolution[0] / self.spec.chunk_resolution[0],
            self.spec.root_resolution[1] / self.spec.chunk_resolution[1],
        ];

        // Copy cells over from chunks that own cells to those that
        // contain the same cells but don't own them.
        // TODO: we won't be able to assume that this is always
        // possible for long, because often most of the chunks
        // won't be loaded. This needs to be hooked up in a more
        // subtle way. :)
        for root_index in 0..ROOT_QUADS {
            let root = Root { index: root_index };
            for z in 0..Z_CHUNKS {
                for y in 0..chunks_per_root[1] {
                    for x in 0..chunks_per_root[0] {
                        let origin = ChunkOrigin::new(
                            CellPos {
                                root: root,
                                x: x * self.spec.chunk_resolution[0],
                                y: y * self.spec.chunk_resolution[1],
                                z: z * self.spec.chunk_resolution[2],
                            },
                            self.spec.root_resolution,
                            self.spec.chunk_resolution,
                        );
                        self.maybe_copy_authoritative_cells(origin);
                    }
                }
            }
        }
    }

    fn maybe_copy_authoritative_cells(&mut self, target_chunk_origin: ChunkOrigin) {
        // BEWARE: MULTIPLE HACKS BELOW to get around borrowck. There has to be
        // a better way around this!

        // Temporarily remove the target chunk from the globe,
        // so that we can simultaneously write to it and read from
        // a bunch of other chunks.
        //
        // TODO: at very least avoid this most of the time by doing a read-only pass over
        // all neighbours and bailing out if we're completely up-to-date.
        let mut target_chunk = match self.chunks.remove(&target_chunk_origin) {
            None => {
                // No worries; the chunk isn't loaded. Do nothing.
                return;
            },
            Some(target_chunk) => target_chunk,
        };

        // Temporarily remove list of neighbours from target chunk
        // so that we can simultaneously read from it and update
        // the chunk's data.
        let mut neighbors = Vec::<super::chunk::Neighbor>::new();
        use std::mem::swap;
        swap(&mut neighbors, &mut target_chunk.authoritative_neighbors);

        // For each of this chunk's neighbors, see if we have up-to-date
        // copies of the data we share with that neighbor. Otherwise,
        // copy it over.
        for neighbor in &mut neighbors {
            let source_chunk = match self.chunks.get(&neighbor.origin) {
                None => {
                    // No worries; the chunk isn't loaded. Do nothing.
                    continue;
                },
                Some(chunk) => chunk,
            };

            if neighbor.last_known_version == source_chunk.version {
                // We're already up-to-date with this neighbor's data; skip.
                continue;
            }

            // Copy over each cell, one by one.
            for target_cell_pos in &neighbor.shared_cells {
                let source_cell_pos: CellPos = PosInOwningRoot::new(*target_cell_pos, self.spec.root_resolution).into();
                let source_cell =
                    *source_chunk.cell(source_cell_pos);
                let target_cell =
                    target_chunk.cell_mut(*target_cell_pos);
                // Copy source -> target.
                *target_cell = source_cell;
            }

            // We're now up-to-date with the neighbor.
            neighbor.last_known_version = source_chunk.version;

            // If we got this far, then it means we needed to update something.
            // So mark this chunk as having its view out-of-date.
            //
            // TODO: this all lacks subtlety; we only actually need to update the
            // destination chunk's data if the dirty cell was on the edge of
            // the chunk. This needs some thought on a good interface for mutating
            // chunks that doesn't allow oopsing this...
            target_chunk.mark_view_as_dirty();
        }

        // Put the list of neighbors back into the chunk.
        swap(&mut neighbors, &mut target_chunk.authoritative_neighbors);

        // Put the target chunk back into the world!
        self.chunks.insert(target_chunk_origin, target_chunk);
    }

    pub fn origin_of_chunk_owning(&self, pos: PosInOwningRoot) -> ChunkOrigin {
        origin_of_chunk_owning(pos, self.spec.root_resolution, self.spec.chunk_resolution)
    }

    // NOTE: chunk returned probably won't _own_ `pos`.
    pub fn origin_of_chunk_in_same_root_containing(&self, pos: CellPos) -> ChunkOrigin {
        // Figure out what chunk this is in.
        origin_of_chunk_in_same_root_containing(pos, self.spec.root_resolution, self.spec.chunk_resolution)
    }

    pub fn find_lowest_cell_containing(
        &self,
        column: CellPos,
        material: Material
    ) -> Option<CellPos> {
        // Translate into owning root, then start at bedrock.
        let mut pos = PosInOwningRoot::new(column, self.spec.root_resolution);
        pos.set_z(0);

        loop {
            let chunk_origin = self.origin_of_chunk_owning(pos);
            let chunk = match self.chunks.get(&chunk_origin) {
                // We may have run out of chunks to inspect.
                // TODO: this may become a problem if we allow infinite
                // or very loose height for planets. Have a limit?
                // Probably only limit to planet height, because if you
                // legitimately have terrain that high, you probably just
                // want to wait to find it!
                None => return None,
                Some(chunk) => chunk,
            };
            let cell = chunk.cell(pos.into());
            if cell.material == material {
                // Yay, we found it!
                return Some(pos.into());
            }
            let new_z = pos.pos().z + 1;
            pos.set_z(new_z);
        }
    }

    pub fn ensure_chunk_view_entities(
        &mut self,
        world: &specs::World,
        globe_entity: specs::Entity,
    ) {
        for chunk in self.chunks.values_mut() {
            // TODO: when we're dynamically destroying chunk views,
            // you'll need to check whether it's still alive when trying
            // to ensure it exists.
            if chunk.view_entity.is_some() {
                continue;
            }
            trace!(self.log, "Making a chunk view"; "origin" => format!("{:?}", chunk.origin));
            let chunk_view = super::ChunkView::new(
                globe_entity,
                chunk.origin,
            );
            // We'll fill it in later.
            let empty_visual = ::render::Visual::new_empty();
            chunk.view_entity = world.create_later_build()
                .with(chunk_view)
                .with(empty_visual)
                // TODO: parent it on the globe when we can do that.
                .with(Spatial::root())
                .build()
                .into();
        }
    }

    /// Most `Chunks`s will have an associated `ChunkView`. Indicate that the
    /// chunk (or something else affecting its visibility) has been modified
    /// since the view was last updated.
    pub fn mark_chunk_views_affected_by_cell_as_dirty(&mut self, pos: CellPos) {
        // Translate into owning root.
        // TODO: wrapper types so we don't have to do
        // this sort of thing defensively!
        // TODO: is it even necessary at all here? All we really care about
        // is consistency in which chunk we look at for the centre cell,
        // and the one.....
        // TODO: really, just rewrite this whole function. It doesn't really work.
        let pos_in_owning_root = PosInOwningRoot::new(pos, self.spec.root_resolution);

        // TODO: this (Vec) is super slow! Replace with a less brain-dead solution.
        // Just committing this one now to patch over a kinda-regression in that
        // the existing bug of not doing this at all just become a lot more
        // obvious now that I'm doing a better job of culling cells.
        let mut dirty_cells: Vec<PosInOwningRoot> = Vec::new();
        dirty_cells.push(pos_in_owning_root);
        dirty_cells.extend(
            Neighbors::new(pos_in_owning_root.into(), self.spec.root_resolution)
                .map(|neighbor_pos| PosInOwningRoot::new(neighbor_pos, self.spec.root_resolution))
        );
        // Gah, filthy hacks. This is to get around not having a way to query for
        // "all chunks containing this cell".
        let mut cells_in_dirty_chunks: Vec<PosInOwningRoot> = Vec::new();
        for dirty_cell in dirty_cells {
            cells_in_dirty_chunks.extend(
                Neighbors::new(dirty_cell.into(), self.spec.root_resolution)
                    .map(|neighbor_pos| PosInOwningRoot::new(neighbor_pos, self.spec.root_resolution))
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

    // TODO: this all lacks subtlety; we only actually need to update the
    // destination chunk's data if the dirty cell was on the edge of
    // the chunk. This needs some thought on a good interface for mutating
    // chunks that doesn't allow oopsing this...
    pub fn increment_chunk_version_for_cell(&mut self, pos: PosInOwningRoot) {
        let chunk_origin = self.origin_of_chunk_owning(pos.into());
        let chunk = self.chunks.get_mut(&chunk_origin)
            .expect("Uh oh, I don't know how to handle chunks that aren't loaded yet.");
        chunk.version += 1;
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
        self.chunks.remove(&chunk_origin)
            .expect("Attempted to remove a chunk that was not loaded")
    }
}

impl<'a> Globe {
    // TODO: this naming is horrible. :)

    pub fn authoritative_cell(
        &'a self,
        pos: PosInOwningRoot,
    ) -> &'a Cell {
        let chunk_origin = self.origin_of_chunk_owning(pos);
        let chunk = self.chunks.get(&chunk_origin)
            .expect("Uh oh, I don't know how to handle chunks that aren't loaded yet.");
        chunk.cell(pos.into())
    }

    pub fn authoritative_cell_mut(
        &'a mut self,
        pos: PosInOwningRoot,
    ) -> &'a mut Cell {
        let chunk_origin = self.origin_of_chunk_owning(pos);
        let chunk = self.chunks.get_mut(&chunk_origin)
            .expect("Uh oh, I don't know how to handle chunks that aren't loaded yet.");
        chunk.cell_mut(pos.into())
    }

    pub fn maybe_non_authoritative_cell(
        &'a self,
        pos: CellPos,
    ) -> &'a Cell {
        let chunk_origin = self.origin_of_chunk_in_same_root_containing(pos);
        let chunk = self.chunks.get(&chunk_origin)
            .expect("Uh oh, I don't know how to handle chunks that aren't loaded yet.");
        chunk.cell(pos)
    }
}

impl specs::Component for Globe {
    type Storage = specs::HashMapStorage<Globe>;
}
