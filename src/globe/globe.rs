use rand;
use rand::Rng;

use super::Root;
use super::chunk::{ Chunk, CellPos, Cell };
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
    pub fn new(spec: Spec) -> Globe {
        let mut globe = Globe {
            spec: spec,
            gen: Gen::new(spec),
            chunks: Vec::new(),
        };
        globe.build_all_chunks();
        globe
    }

    pub fn new_example() -> Globe {
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
            }
        )
    }

    pub fn spec(&self) -> Spec {
        self.spec
    }

    pub fn build_all_chunks(&mut self) {
        // Calculate how many chunks to a root in each direction in (x, y).
        let chunks_per_root = [
            self.spec.root_resolution[0] / self.spec.chunk_resolution[0],
            self.spec.root_resolution[1] / self.spec.chunk_resolution[1],
        ];
        for root_index in 0..ROOT_QUADS {
            let root = Root { index: root_index };
            // TODO: how many to build high?
            for z in 0..5 {
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
    }

    pub fn build_chunk(&mut self, origin: CellPos) {
        let mut cells: Vec<Cell> = Vec::new();
        // Include cells _on_ the far edge of the chunk;
        // even though we don't own them we'll need to draw part of them.
        let end_x = origin.x + self.spec.chunk_resolution[0] + 1;
        let end_y = origin.y + self.spec.chunk_resolution[1] + 1;
        let end_z = origin.z + self.spec.chunk_resolution[2] + 1;
        for cell_z in origin.z..end_z {
            for cell_y in origin.y..end_y {
                for cell_x in origin.x..end_x {
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
}
