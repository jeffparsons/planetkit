use std::collections::HashMap;

use specs;

use super::{ origin_of_chunk_owning, origin_of_chunk_in_same_root_containing };
use super::{ CellPos, PosInOwningRoot, ChunkOrigin };
use super::Neighbors;
use super::chunk::{ Chunk, Cell, Material };
use super::spec::Spec;
use super::gen::Gen;

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
        }
    }

    pub fn new_example() -> Globe {
        Globe::new(
            Spec {
                seed: 14,
                floor_radius: 25.0,
                ocean_radius: 66.6,
                block_height: 0.65,
                root_resolution: [64, 128],
                // Chunks should probably be taller, but short chunks are a bit
                // better for now in exposing bugs visually.
                chunk_resolution: [16, 16, 4],
            },
        )
    }

    pub fn new_earth_scale_example() -> Globe {
        let ocean_radius = 6_371_000.0;
        // TODO: actually more like 60_000 when we know how to:
        // - Unload chunks properly
        // - Start with a guess about the z-position of the player
        //   so we don't have to start at bedrock and search up.
        let crust_depth = 60.0;
        let floor_radius = ocean_radius - crust_depth;
        Globe::new(
            Spec {
                // TODO: This only coincidentally puts you on land.
                // Implement deterministic (PRNG) land finding so that the seed does not matter.
                seed: 14,
                floor_radius: floor_radius,
                ocean_radius: ocean_radius,
                block_height: 0.65,
                root_resolution: [8388608, 16777216],
                // Chunks should probably be taller, but short chunks are a bit
                // better for now in exposing bugs visually.
                chunk_resolution: [16, 16, 4],
            },
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
        // Oh god, this is so horrible. Please forgive this
        // temporary hack until I figure out what I'm actually
        // doing with this. I want to destroy the whole interface,
        // so I'm not going to bother making this good for now.
        //
        // TODO: burn it with fire
        let all_chunk_origins: Vec<ChunkOrigin> = self.chunks
            .values()
            .map(|chunk| chunk.origin)
            .collect();
        for chunk_origin in all_chunk_origins {
            self.maybe_copy_authoritative_cells(chunk_origin);
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
                    let cell_pos = CellPos {
                        root: origin.pos().root,
                        x: cell_x,
                        y: cell_y,
                        z: cell_z,
                    };
                    let mut cell = self.gen.cell_at(cell_pos);
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
    pub fn ensure_chunk_present(&mut self, chunk_origin: ChunkOrigin) {
        if self.chunk_at(chunk_origin).is_some() {
            return;
        }
        self.load_or_build_chunk(chunk_origin);

        // TODO: slow, oh gods, don't do this.
        // But for now, it will at least correctly copy in/out
        // any authoritative cells.
        self.copy_all_authoritative_cells();
    }

    // TODO: this is not sufficient for finding a suitable place
    // to put a cell dweller; i.e. we need something that randomly
    // samples positions to find a column with land at the top,
    // probably by using the `Gen` to find an approximate location,
    // and then working up and down at the same time to find the
    // closest land to the "surface".
    pub fn find_lowest_cell_containing(
        &mut self,
        column: CellPos,
        material: Material
    ) -> Option<CellPos> {
        // Translate into owning root, then start at bedrock.
        let mut pos = PosInOwningRoot::new(column, self.spec.root_resolution);
        pos.set_z(0);

        loop {
            // TODO: cursor doesn't guarantee you're reading authoritative data.
            // Do we care about that? Do we just need to make sure that "ensure chunk"
            // loads any other chunks that might be needed? But gah, then you're going to
            // have a chain reaction, and load ALL chunks. Maybe it's Cursor's
            // responsibility, then. TODO: think about this. :)
            //
            // Maybe you need a special kind of cursor. That only looks at owned cells
            // and automatically updates itself whenever you set its position.
            let chunk_origin = self.origin_of_chunk_owning(pos);
            self.ensure_chunk_present(chunk_origin);
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
}

impl<'a> Globe {
    pub fn chunk_at(&'a self, chunk_origin: ChunkOrigin) -> Option<&'a Chunk> {
        self.chunks.get(&chunk_origin)
    }

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
