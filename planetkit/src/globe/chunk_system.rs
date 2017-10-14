use specs;
use specs::{ReadStorage, WriteStorage};
use specs::Entities;
use slog::Logger;

use grid::PosInOwningRoot;
use super::{Globe, ChunkOrigin};
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
    pub fn new(parent_log: &Logger) -> ChunkSystem {
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

    fn unload_excess_chunks_if_necessary<'a>(
        &mut self,
        globe: &mut Globe,
        globe_entity: specs::Entity,
        cds: &specs::ReadStorage<'a, CellDweller>,
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
        let one_true_cd = match cds.join()
            .filter(|cd| cd.globe_entity == Some(globe_entity))
            .next() {
            Some(cd) => cd,
            // There are no cell dwellers, so no interesting terrain.
            // (If a tree falls in a forest...)
            None => return,
        };
        let one_true_cd_pos = one_true_cd
            .real_transform_without_setting_clean()
            .translation
            .vector;

        // Unload the most distant chunks.
        //
        // TODO: Don't allocate memory all the time here.
        // At very least use a persistent scratch buffer instead
        // of allocating every time!
        let mut chunk_distances: Vec<(ChunkOrigin, f64)> = globe
            .chunks()
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
        chunk_distances.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).expect(
                "All chunk origins and CellDwellers should be real distances from each other!",
            )
        });
        let chunks_to_remove = self.max_chunks_loaded_per_globe - self.cull_chunks_down_to;
        chunk_distances.truncate(chunks_to_remove);

        for (chunk_origin, _distance) in chunk_distances {
            globe.remove_chunk(chunk_origin);
        }
    }

    fn ensure_essential_chunks_for_cell_dweller_present<'a>(
        &mut self,
        cd: &CellDweller,
        globes: &mut specs::WriteStorage<'a, Globe>,
    ) {
        if let Some(globe_entity) = cd.globe_entity {
            // Get the associated globe, complaining loudly if we fail.
            // TODO: this is becoming a common pattern; factor out.
            let globe = match globes.get_mut(globe_entity) {
                Some(globe) => globe,
                None => {
                    warn!(
                        self.log,
                        "The globe associated with this CellDweller is not alive! Can't proceed!"
                    );
                    return;
                }
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
            globe.ensure_chunk_present(chunk_origin);
            let accessible_chunks = {
                use super::globe::GlobeGuts;
                let chunk = globe.chunks().get(&chunk_origin).expect(
                    "We just ensured this chunk is loaded.",
                );
                // TODO: Gah, such slow!
                chunk.accessible_chunks.clone()
            };
            for accessible_chunk_origin in accessible_chunks {
                globe.ensure_chunk_present(accessible_chunk_origin);

                // Repeat this from each immediately accessible chunk.
                let next_level_accessible_chunks = {
                    use super::globe::GlobeGuts;
                    let chunk = globe.chunks().get(&accessible_chunk_origin).expect(
                        "We just ensured this chunk is loaded.",
                    );
                    // TODO: Gah, such slow!
                    chunk.accessible_chunks.clone()
                };
                for next_level_accessible_chunk_origin in next_level_accessible_chunks {
                    globe.ensure_chunk_present(next_level_accessible_chunk_origin);
                }
            }
        }
    }
}

impl<'a> specs::System<'a> for ChunkSystem {
    type SystemData = (Entities<'a>, WriteStorage<'a, Globe>, ReadStorage<'a, CellDweller>);

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;

        let (entities, mut globes, cds) = data;

        // If we have too many chunks loaded, then unload some of them.
        for (mut globe, globe_entity) in (&mut globes, &*entities).join() {
            self.unload_excess_chunks_if_necessary(&mut globe, globe_entity, &cds);
        }
        // Make sure the chunks under/near the player are present.
        for cd in cds.join() {
            self.ensure_essential_chunks_for_cell_dweller_present(cd, &mut globes);
        }
    }
}
