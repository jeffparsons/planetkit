use specs;
use slog::Logger;

use chrono::Duration;

use rand;
use rand::Rng;

use types::*;
use super::{ Globe, CellPos, ChunkOrigin };
use super::chunk::{ Chunk, Cell };
use super::Root;

// TODO: lift to module level.
const ROOT_QUADS: u8 = 5;

// TODO: remove me
const Z_CHUNKS: i64 = 5;

/// Loads and unloads `Chunk`s for a `Globe`.
///
/// The `Chunk`s may be loaded from disk, or generated fresh if
/// they have never existed before.
pub struct ChunkSystem {
    log: Logger,
    // TEMP
    have_built_chunks: bool,
}

impl ChunkSystem {
    pub fn new(
        parent_log: &Logger,
    ) -> ChunkSystem {
        ChunkSystem {
            log: parent_log.new(o!()),
            have_built_chunks: false,
        }
    }

    pub fn build_all_chunks(&mut self, globe: &mut Globe) {
        use super::globe::GlobeGuts;

        // Calculate how many chunks to a root in each direction in (x, y).
        let spec = globe.spec();
        let chunks_per_root = [
            spec.root_resolution[0] / spec.chunk_resolution[0],
            spec.root_resolution[1] / spec.chunk_resolution[1],
        ];

        debug!(self.log, "Making chunks...");

        let dt = Duration::span(|| {
            for root_index in 0..ROOT_QUADS {
                let root = Root { index: root_index };
                for z in 0..Z_CHUNKS {
                    for y in 0..chunks_per_root[1] {
                        for x in 0..chunks_per_root[0] {
                            let origin = ChunkOrigin::new(
                                CellPos {
                                    root: root,
                                    x: x * spec.chunk_resolution[0],
                                    y: y * spec.chunk_resolution[1],
                                    z: z * spec.chunk_resolution[2],
                                },
                                spec.root_resolution,
                                spec.chunk_resolution,
                            );
                            self.build_chunk(globe, origin);
                        }
                    }
                }
            }
        });

        debug!(self.log, "Finished making chunks"; "chunks" => globe.chunks().len(), "dt" => format!("{}", dt));

        // TODO: this is _not_ going to fly once we're trickling the
        // chunks in over time...
        globe.copy_all_authoritative_cells();

        self.have_built_chunks = true;
    }

    // TODO: rip all this out into a system.
    pub fn build_chunk(&mut self, globe: &mut Globe, origin: ChunkOrigin) {
        let spec = globe.spec();

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
                    let mut cell = globe.gen.cell_at(cell_pos);
                    // Temp hax?
                    let mut rng = rand::thread_rng();
                    cell.shade = 1.0 - 0.5 * rng.next_f32();
                    cells.push(cell);
                }
            }
        }
        globe.add_chunk(Chunk::new(
            origin,
            cells,
            spec.root_resolution,
            spec.chunk_resolution,
        ));
    }
}

impl specs::System<TimeDelta> for ChunkSystem {
    fn run(&mut self, arg: specs::RunArg, _dt: TimeDelta) {
        use specs::Join;
        let (mut globes,) = arg.fetch(|w| {
            (w.write::<Globe>(),)
        });

        for globe in (&mut globes).iter() {
            // TEMP; just factoring the dumb "build everything" logic out into
            // a system before making it do interesting things.
            if !self.have_built_chunks {
                self.build_all_chunks(globe);
            }
        }
    }
}
