use specs;

use chrono::Duration;

use slog::Logger;

use rand;
use rand::Rng;

use super::{ pos_in_owning_root, origin_of_chunk_owning };
use super::Root;
use super::CellPos;
use super::chunk::{ Chunk, Cell, Material };
use super::spec::Spec;
use super::gen::Gen;
use ::Spatial;

const ROOT_QUADS: u8 = 5;

// TODO: split out a WorldGen type that handles all the procedural
// generation, because none of that really needs to be tangled
// with the realised Globe.
pub struct Globe {
    spec: Spec,
    gen: Gen,
    // TODO: figure out what structure to store these in.
    // You'll never have all chunks loaded in the real world.
    //
    // TODO: you'll probably also want to store some lower-res
    // pseudo-chunks for rendering planets at a distance.
    // But maybe you can put that off?
    chunks: Vec<Chunk>,
    log: Logger,
}

// Allowing sibling modules to reach into semi-private parts
// of the Globe struct.
pub trait GlobeGuts<'a> {
    fn chunks(&'a self) -> &'a Vec<Chunk>;
    fn chunks_mut(&'a mut self) -> &'a mut Vec<Chunk>;
}

impl<'a> GlobeGuts<'a> for Globe {
    fn chunks(&'a self) -> &'a Vec<Chunk> {
        &self.chunks
    }

    fn chunks_mut(&'a mut self) -> &'a mut Vec<Chunk> {
        &mut self.chunks
    }
}

impl Globe {
    pub fn new(spec: Spec, parent_log: &Logger) -> Globe {
        let mut globe = Globe {
            spec: spec,
            gen: Gen::new(spec),
            chunks: Vec::new(),
            log: parent_log.new(o!()),
        };
        globe.build_all_chunks();
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
            },
            parent_log,
        )
    }

    pub fn spec(&self) -> Spec {
        self.spec
    }

    pub fn build_all_chunks(&mut self) {
        // TODO: how many to build high?
        const Z_CHUNKS: i64 = 5;

        // Calculate how many chunks to a root in each direction in (x, y).
        let chunks_per_root = [
            self.spec.root_resolution[0] / self.spec.chunk_resolution[0],
            self.spec.root_resolution[1] / self.spec.chunk_resolution[1],
        ];

        debug!(self.log, "Making chunks...");

        let dt = Duration::span(|| {
            for root_index in 0..ROOT_QUADS {
                let root = Root { index: root_index };
                for z in 0..Z_CHUNKS {
                    for y in 0..chunks_per_root[1] {
                        for x in 0..chunks_per_root[0] {
                            let origin = CellPos {
                                root: root,
                                x: x * self.spec.chunk_resolution[0],
                                y: y * self.spec.chunk_resolution[1],
                                z: z * self.spec.chunk_resolution[2],
                            };
                            self.build_chunk(origin);
                        }
                    }
                }
            }
        });

        debug!(self.log, "Finished making chunks"; "chunks" => self.chunks.len(), "dt" => format!("{}", dt));

        // TODO: this is _not_ going to fly once we're trickling the
        // chunks in over time...
        self.copy_all_authoritative_cells();
    }

    pub fn build_chunk(&mut self, origin: CellPos) {
        let mut cells: Vec<Cell> = Vec::new();
        // Include cells _on_ the far edge of the chunk;
        // even though we don't own them we'll need to draw part of them.
        let end_x = origin.x + self.spec.chunk_resolution[0];
        let end_y = origin.y + self.spec.chunk_resolution[1];
        // Chunks don't share cells in the z-direction,
        // but do in the x- and y-directions.
        let end_z = origin.z + self.spec.chunk_resolution[2] - 1;
        for cell_z in origin.z..(end_z + 1) {
            for cell_y in origin.y..(end_y + 1) {
                for cell_x in origin.x..(end_x + 1) {
                    let cell_pos = CellPos {
                        root: origin.root,
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
        self.chunks.push(Chunk::new(
            origin,
            cells,
            self.spec.root_resolution,
            self.spec.chunk_resolution,
        ));
    }

    // TODO: there's no way this should be public.
    // Replace with a better interface for mutating cell content
    // that automatically ensures that all neighbouring chunks
    // get updated, etc.
    pub fn copy_all_authoritative_cells(&mut self) {
        // TODO: how many to build high?
        const Z_CHUNKS: i64 = 5;

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
                        let origin = CellPos {
                            root: root,
                            x: x * self.spec.chunk_resolution[0],
                            y: y * self.spec.chunk_resolution[1],
                            z: z * self.spec.chunk_resolution[2],
                        };
                        self.maybe_copy_authoritative_cells(origin);
                    }
                }
            }
        }
    }

    fn maybe_copy_authoritative_cells(&mut self, target_chunk_origin: CellPos) {
        let target_chunk_index = match self.index_of_chunk_at(target_chunk_origin) {
            None => {
                // No worries; the chunk isn't loaded. Do nothing.
                return;
            },
            Some(target_chunk_index) => target_chunk_index,
        };

        // BEWARE: MULTIPLE HACKS BELOW to get around borrowck. There has to be
        // a better way around this!

        // Temporarily remove the target chunk from the globe,
        // so that we can simultaneously write to it and read from
        // a bunch of other chunks.
        let mut target_chunk = self.chunks.swap_remove(target_chunk_index);

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
            let source_chunk_index = match self.index_of_chunk_at(neighbor.origin) {
                None => {
                    // No worries; the chunk isn't loaded. Do nothing.
                    continue;
                },
                Some(source_chunk_index) => source_chunk_index,
            };

            let source_chunk = &self.chunks[source_chunk_index];
            if neighbor.last_known_version == source_chunk.version {
                // We're already up-to-date with this neighbor's data; skip.
                continue;
            }

            // Copy over each cell, one by one.
            for target_cell_pos in &neighbor.shared_cells {
                let source_cell_pos = pos_in_owning_root(*target_cell_pos, self.spec.root_resolution);
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
        self.chunks.push(target_chunk);
    }

    // Returns None if given coordinates of a cell in a chunk we don't have loaded,
    // or could never load because `origin` is not a valid chunk origin.
    pub fn index_of_chunk_at(&self, origin: CellPos) -> Option<usize> {
        self.chunks.iter().position(|chunk| chunk.origin == origin)
    }

    // Returns None if given coordinates of a cell in a chunk we don't have loaded.
    //
    // Translates given `pos` into its owning root if necessary.
    // (TODO: make a version that trusts it's already in its owning root,
    // because this will be quite wasteful if we can already trust it.
    // Maybe represent this with types. We also probably want to sometimes
    // get a reference to cells that aren't the authoritative source
    // for that position.)
    pub fn index_of_chunk_owning(&self, mut pos: CellPos) -> Option<usize> {
        // Translate into owning root.
        pos = pos_in_owning_root(pos, self.spec.root_resolution);

        // Figure out what chunk this is in.
        let chunk_origin = origin_of_chunk_owning(pos, self.spec.root_resolution, self.spec.chunk_resolution);

        self.index_of_chunk_at(chunk_origin)
    }

    pub fn find_lowest_cell_containing(
        &self,
        column: CellPos,
        material: Material
    ) -> Option<CellPos> {
        // Translate into owning root, then start at bedrock.
        let mut pos = pos_in_owning_root(column, self.spec.root_resolution);
        pos.z = 0;

        loop {
            let chunk_index = match self.index_of_chunk_owning(pos) {
                // We may have run out of chunks to inspect.
                // TODO: this may become a problem if we allow infinite
                // or very loose height for planets. Have a limit?
                // Probably only limit to planet height, because if you
                // legitimately have terrain that high, you probably just
                // want to wait to find it!
                None => return None,
                Some(chunk_index) => chunk_index,
            };

            let cell = self.chunks[chunk_index].cell(pos);
            if cell.material == material {
                // Yay, we found it!
                return pos.into();
            }
            pos.z += 1;
        }
    }

    pub fn ensure_chunk_view_entities(
        &mut self,
        world: &specs::World,
        globe_entity: specs::Entity,
    ) {
        for chunk in &mut self.chunks {
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
    /// chunk has been modified since the view was last updated.
    pub fn mark_chunk_view_as_dirty(&mut self, mut pos: CellPos) {
        // Translate into owning root.
        // TODO: wrapper types so we don't have to do
        // this sort of thing defensively!
        pos = pos_in_owning_root(pos, self.spec.root_resolution);
        let chunk_index = self.index_of_chunk_owning(pos)
            .expect("Uh oh, I don't know how to handle chunks that aren't loaded yet.");
        self.chunks[chunk_index].mark_view_as_dirty();
    }

    // TODO: this all lacks subtlety; we only actually need to update the
    // destination chunk's data if the dirty cell was on the edge of
    // the chunk. This needs some thought on a good interface for mutating
    // chunks that doesn't allow oopsing this...
    pub fn increment_chunk_version_for_cell(&mut self, mut pos: CellPos) {
        // Translate into owning root.
        // TODO: wrapper types so we don't have to do
        // this sort of thing defensively!
        pos = pos_in_owning_root(pos, self.spec.root_resolution);
        let chunk_index = self.index_of_chunk_owning(pos)
            .expect("Uh oh, I don't know how to handle chunks that aren't loaded yet.");
        self.chunks[chunk_index].version += 1;
    }
}

impl<'a> Globe {
    pub fn cell(
        &'a self,
        mut pos: CellPos,
    ) -> &'a Cell {
        // Translate into owning root.
        // TODO: wrapper types so we don't have to do
        // this sort of thing defensively!
        pos = pos_in_owning_root(pos, self.spec.root_resolution);
        let chunk_index = self.index_of_chunk_owning(pos)
            .expect("Uh oh, I don't know how to handle chunks that aren't loaded yet.");
        self.chunks[chunk_index].cell(pos)
    }

    pub fn cell_mut(
        &'a mut self,
        mut pos: CellPos,
    ) -> &'a mut Cell {
        // Translate into owning root.
        // TODO: wrapper types so we don't have to do
        // this sort of thing defensively!
        pos = pos_in_owning_root(pos, self.spec.root_resolution);
        let chunk_index = self.index_of_chunk_owning(pos)
            .expect("Uh oh, I don't know how to handle chunks that aren't loaded yet.");
        self.chunks[chunk_index].cell_mut(pos)
    }
}

impl specs::Component for Globe {
    type Storage = specs::HashMapStorage<Globe>;
}
