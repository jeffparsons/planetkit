use std::ops::{ Deref, DerefMut };

use specs;
use slog::Logger;

use rand;
use rand::Rng;

use types::*;
use super::{ Globe, CellPos, ChunkOrigin, PosInOwningRoot };
use super::chunk::{ Chunk, Cell, Material };
use cell_dweller::CellDweller;

/// Loads and unloads `Chunk`s for a `Globe`.
///
/// The `Chunk`s may be loaded from disk, or generated fresh if
/// they have never existed before.
pub struct ChunkSystem {
    log: Logger,
    // When we go higher than this many chunks loaded...
    max_chunks_loaded_per_globe: usize,
    // ...we will unload chunks to leave only this many.
    // This is a kind of hysteresis. I haven't yet validated
    // that this actually improves performance _at all_.
    cull_chunks_down_to: usize,
}

impl ChunkSystem {
    pub fn new(
        parent_log: &Logger,
    ) -> ChunkSystem {
        ChunkSystem {
            log: parent_log.new(o!()),
            // TODO: accept as arguments.
            //
            // There appears to be at least ~110
            // loaded at a minimum the way I have it at the moment;
            // have to be super careful to get these numbers right
            // so we don't unnecessarily churn chunks.
            //
            // TODO: how to make sure this is automatically right?
            max_chunks_loaded_per_globe: 200,
            cull_chunks_down_to: 150,
        }
    }

    // TODO: rewrite this to build or load a chunk, rename it,
    // and revisit all its clients.
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
    // TODO: add a mechanism for temporarily flagging chunks as essential,
    // and then "find_lowest_cell_containing" can flag those chunks as it
    // walks up through the globe.
    pub fn ensure_chunk_present(&mut self, globe: &mut Globe, chunk_origin: ChunkOrigin) {
        if globe.chunk_at(chunk_origin).is_some() {
            return;
        }
        self.build_chunk(globe, chunk_origin);

        // TODO: slow, oh gods, don't do this.
        // But for now, it will at least correctly copy in/out
        // any authoritative cells.
        globe.copy_all_authoritative_cells();
    }

    // TODO: this is not sufficient for finding a suitable place
    // to put a cell dweller; i.e. we need something that randomly
    // samples positions to find a column with land at the top,
    // probably by using the `Gen` to find an approximate location,
    // and then working up and down at the same time to find the
    // closest land to the "surface".
    pub fn find_lowest_cell_containing(
        &mut self,
        globe: &mut Globe,
        column: CellPos,
        material: Material
    ) -> Option<CellPos> {
        use super::globe::GlobeGuts;

        // Translate into owning root, then start at bedrock.
        let mut pos = PosInOwningRoot::new(column, globe.spec().root_resolution);
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
            let chunk_origin = globe.origin_of_chunk_owning(pos);
            self.ensure_chunk_present(globe, chunk_origin);
            let chunk = match globe.chunks().get(&chunk_origin) {
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

    fn unload_excess_chunks_if_necessary<
        A: Deref<Target = specs::Allocator>,
        Cd: Deref<Target = specs::MaskedStorage<CellDweller>>,
    >(
        &mut self,
        globe: &mut Globe,
        globe_entity: specs::Entity,
        cds: &specs::Storage<CellDweller, A, Cd>,
    ) {
        use super::globe::GlobeGuts;

        if globe.chunks().len() < self.max_chunks_loaded_per_globe {
            // We're under the limit; nothing to do.
            return;
        }

        // TEMP: assume only one cell dweller per globe.
        // TODO: proper entities/points/volumes of interest system.
        use specs::Join;
        // Use the first CellDweller we find on this Globe.
        let one_true_cd = match cds.iter()
            .filter(|cd| cd.globe_entity == Some(globe_entity))
            .next()
        {
            Some(cd) => cd,
            // There are no cell dwellers, so no interesting terrain.
            // (If a tree falls in a forest...)
            None => return,
        };
        let one_true_cd_pos = one_true_cd.real_transform_without_setting_clean()
            .translation.vector;

        // Unload the most distant chunks.
        //
        // TODO: Don't allocate memory all the time here.
        // At very least use a persistent scratch buffer instead
        // of allocating every time!
        let mut chunk_distances: Vec<(ChunkOrigin, f64)> = globe.chunks()
            .keys()
            .map(|chunk_origin| {
                // TODO: don't use chunk origin; use the middle cell,
                // or otherwise whatever the closest corner is.
                // Or even a bounding sphere.
                // (Cache this per Chunk).
                let chunk_origin_pos = globe.spec().cell_bottom_center(*chunk_origin.pos());
                let distance_from_one_true_cd = (one_true_cd_pos - chunk_origin_pos.coords).norm();
                (*chunk_origin, distance_from_one_true_cd)
            })
            .collect();
        // Farthest away chunks come first.
        chunk_distances.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("All chunk origins and CellDwellers should be real distances from each other!"));
        let chunks_to_remove = self.max_chunks_loaded_per_globe - self.cull_chunks_down_to;
        chunk_distances.truncate(chunks_to_remove);

        for (chunk_origin, _distance) in chunk_distances {
            globe.remove_chunk(chunk_origin);
        }
    }

    fn ensure_essential_chunks_for_cell_dweller_present<
        A: Deref<Target = specs::Allocator>,
        Gd: DerefMut<Target = specs::MaskedStorage<Globe>>,
    >(
        &mut self,
        cd: &CellDweller,
        globes: &mut specs::Storage<Globe, A, Gd>,
    ) {
        if let Some(globe_entity) = cd.globe_entity {
            // Get the associated globe, complaining loudly if we fail.
            // TODO: this is becoming a common pattern; factor out.
            let mut globe = match globes.get_mut(globe_entity) {
                Some(globe) => globe,
                None => {
                    warn!(self.log, "The globe associated with this CellDweller is not alive! Can't proceed!");
                    return;
                },
            };

            // TODO: throttle, and do in background.
            // (Except that the truly essential chunks really do need to be loaded _now_.)

            // TODO: see remarks in `Chunk::list_accessible_chunks`
            // about this actually being an inappropriate way to approach
            // this problem; we'll load a bunch of chunks we don't need to yet
            // in a desperate attempt to not miss the ones we do need.

            // Load all the chunks that we could possibly try
            // to move into from this chunk within two steps.
            //
            // Takes into account that a single user action could lead to
            // multiple cell jumps, e.g., stepping up a small ledge.
            //
            // TODO: this is all a bit finicky and fragile.
            let cd_pos_in_owning_root = PosInOwningRoot::new(cd.pos, globe.spec().root_resolution);
            let chunk_origin = globe.origin_of_chunk_owning(cd_pos_in_owning_root);
            self.ensure_chunk_present(globe, chunk_origin);
            let accessible_chunks = {
                use super::globe::GlobeGuts;
                let chunk = globe.chunks().get(&chunk_origin)
                    .expect("We just ensured this chunk is loaded.");
                // TODO: Gah, such slow!
                chunk.accessible_chunks.clone()
            };
            for accessible_chunk_origin in accessible_chunks {
                self.ensure_chunk_present(globe, accessible_chunk_origin);

                // Repeat this from each immediately accessible chunk.
                let next_level_accessible_chunks = {
                    use super::globe::GlobeGuts;
                    let chunk = globe.chunks().get(&accessible_chunk_origin)
                        .expect("We just ensured this chunk is loaded.");
                    // TODO: Gah, such slow!
                    chunk.accessible_chunks.clone()
                };
                for next_level_accessible_chunk_origin in next_level_accessible_chunks {
                    self.ensure_chunk_present(globe, next_level_accessible_chunk_origin);
                }
            }
        }
    }
}

impl specs::System<TimeDelta> for ChunkSystem {
    fn run(&mut self, arg: specs::RunArg, _dt: TimeDelta) {
        use specs::Join;
        let (mut globes, cds, entities) = arg.fetch(|w| {
            (w.write::<Globe>(), w.read::<CellDweller>(), w.entities())
        });

        // If we have too many chunks loaded, then unload some of them.
        for (mut globe, globe_entity) in (&mut globes, &entities).iter() {
            self.unload_excess_chunks_if_necessary(&mut globe, globe_entity, &cds);
        }

        for cd in cds.iter() {
            self.ensure_essential_chunks_for_cell_dweller_present(cd, &mut globes);
        }
    }
}
