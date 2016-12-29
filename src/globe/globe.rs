use chrono::Duration;

use slog::Logger;

use rand;
use rand::Rng;

use super::Root;
use super::CellPos;
use super::chunk::{ Chunk, Cell, Material };
use super::spec::Spec;
use super::gen::Gen;

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
}

impl<'a> GlobeGuts<'a> for Globe {
    fn chunks(&'a self) -> &'a Vec<Chunk> {
        &self.chunks
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
                        self.copy_authoritative_cells(origin);
                    }
                }
            }
        }
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
        self.chunks.push(Chunk {
            origin: origin,
            cells: cells,
            resolution: self.spec.chunk_resolution,
        });
    }

    fn copy_authoritative_cells(&mut self, target_chunk_origin: CellPos) {
        let origin = target_chunk_origin;
        let target_chunk_index = self.index_of_chunk_at(origin)
            .expect("Uh oh, I don't know how to handle chunks that aren't loaded yet.");
        let end_x = origin.x + self.spec.chunk_resolution[0];
        let end_y = origin.y + self.spec.chunk_resolution[1];
        // Chunks don't share cells in the z-direction,
        // but do in the x- and y-directions.
        let end_z = origin.z + self.spec.chunk_resolution[2] - 1;
        // TODO: this would be really easy to do way more efficiently.
        // Specifically, we could iterate over _exactly_ the cells we know
        // we'll need to copy, and know _exactly_ what chunks they're from.
        for cell_z in origin.z..(end_z + 1) {
            for cell_y in origin.y..(end_y + 1) {
                for cell_x in origin.x..(end_x + 1) {
                    let target_cell_pos = CellPos {
                        root: origin.root,
                        x: cell_x,
                        y: cell_y,
                        z: cell_z,
                    };
                    // TODO: Suuuuper inefficient doing this in the hot loop.
                    // Do this in a less brain-dead way. :)
                    let source_cell_pos = self.spec.pos_in_owning_root(target_cell_pos);
                    let source_chunk_index = self.index_of_chunk_owning(source_cell_pos)
                        .expect("Uh oh, I don't know how to handle chunks that aren't loaded yet.");
                    let source_cell =
                        *self.chunks[source_chunk_index].cell(source_cell_pos);
                    let target_cell =
                        self.chunks[target_chunk_index].cell_mut(target_cell_pos);
                    // Copy source-target.
                    *target_cell = source_cell;
                }
            }
        }
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
        pos = self.spec.pos_in_owning_root(pos);

        // Figure out what chunk this is in.
        let end_x = self.spec.root_resolution[0];
        let end_y = self.spec.root_resolution[1];
        let chunk_res = self.spec.chunk_resolution;
        let last_chunk_x = (end_x / chunk_res[0] - 1) * chunk_res[0];
        let last_chunk_y = (end_y / chunk_res[1] - 1) * chunk_res[1];
        // Cells aren't shared by chunks in the z-direction, so the z-origin
        // is the same across all cases. Small mercies.
        let chunk_origin_z = pos.z / chunk_res[2] * chunk_res[2];
        let chunk_origin = if pos.x == 0 && pos.y == 0 {
            // Chunk at (0, 0) owns north pole.
            CellPos {
                root: pos.root,
                x: 0,
                y: 0,
                z: chunk_origin_z,
            }
        } else if pos.x == end_x && pos.y == end_y {
            // Chunk at (last_chunk_x, last_chunk_y) owns south pole.
            CellPos {
                root: pos.root,
                x: last_chunk_x,
                y: last_chunk_y,
                z: chunk_origin_z,
            }
        } else {
            // Chunks own cells on their edge at `x == 0`, and their edge at `y == chunk_res`.
            // The cells on other edges belong to adjacent chunks.
            let chunk_origin_x = pos.x / chunk_res[0] * chunk_res[0];
            // Shift everything down by on in y-direction.
            let chunk_origin_y = (pos.y - 1) / chunk_res[1] * chunk_res[1];
            CellPos {
                root: pos.root,
                x: chunk_origin_x,
                y: chunk_origin_y,
                z: chunk_origin_z,
            }
        };

        self.index_of_chunk_at(chunk_origin)
    }

    pub fn find_lowest_cell_containing(
        &self,
        column: CellPos,
        material: Material
    ) -> Option<CellPos> {
        // Translate into owning root, then start at bedrock.
        let mut pos = self.spec.pos_in_owning_root(column);
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
}
