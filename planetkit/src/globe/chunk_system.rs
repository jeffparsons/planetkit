use slog::Logger;
use specs;
use specs::Entities;
use specs::{ReadStorage, WriteStorage};

use super::{ChunkOrigin, Globe};
use crate::cell_dweller::CellDweller;
use crate::grid::PosInOwningRoot;

// NOTE: this is currently all pretty awful. See comments throughout.

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
    //
    // TODO: instead have a budget for chunks, have the chunk
    // creation logic know what it is and _target_ it when creating
    // new chunks, so that we never end up with thrashing by
    // creating too many then immediately deleting them repeatedly
    // each frame. (You should only go over if you won't do it again
    // immediately after cleaning up.) Then also complain loudly
    // if there's not enough budget left for the really essential chunks
    // for a given CellDweller.
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
            // (See comments above; have a budget, work to that.)
            // These values were originally 200 and 150 before I bumped
            // it up to allow for 3 players. This solution really
            // isn't going to fly very long. It's time to decouple what
            // chunk _views_ exist from what chunks exist, so we can
            // load the essential chunks for each player character,
            // and the desirable views for each client. (Etc.)
            max_chunks_loaded_per_globe: 300,
            cull_chunks_down_to: 250,
        }
    }

    fn unload_excess_chunks_if_necessary<'a>(
        &mut self,
        globe: &mut Globe,
        globe_entity: specs::Entity,
        cds: &specs::ReadStorage<'a, CellDweller>,
    ) {
        use super::globe::GlobeGuts;

        if globe.chunks().len() <= self.max_chunks_loaded_per_globe {
            // We're under the limit; nothing to do.
            return;
        }

        // Get all the CellDweller positions relative to the globe.
        // We don't care which CellDweller is which, so just store
        // them as a Vec of points.
        use specs::Join;
        let cd_positions: Vec<_> = cds.join()
            // Only consider CellDwellers from this globe.
            .filter(|cd| cd.globe_entity == Some(globe_entity))
            .map(|cd| {
                cd.real_transform_without_setting_clean().translation.vector
            })
            .collect();

        // There are no cell dwellers, so no interesting terrain.
        // (If a tree falls in a forest...)
        if cd_positions.len() == 0 {
            return;
        }

        // Unload the chunks that are most distant from their nearest CellDweller.
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
                let distance_from_closest_cd = cd_positions.iter()
                    // TODO: norm_squared; it'll be quicker.
                    .map(|cd_pos| (cd_pos - chunk_origin_pos.coords).norm())
                    .min_by(|a, b| a.partial_cmp(b).expect("Really shouldn't be possible to get NaN etc. here"))
                    .expect("We already ensured there is at least one CellDweller");
                (*chunk_origin, distance_from_closest_cd)
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

            // TODO: this is also just plain wrong.
            // You don't need the neighbouring chunks of the neighbouring chunks.
            // You just need all the chunks containing neighbouring cells of
            // neighbouring cells. No wonder there are so many chunks loaded
            // at the moment. :)
            let cd_pos_in_owning_root = PosInOwningRoot::new(cd.pos, globe.spec().root_resolution);
            let chunk_origin = globe.origin_of_chunk_owning(cd_pos_in_owning_root);
            globe.ensure_chunk_present(chunk_origin);
            let accessible_chunks = {
                use super::globe::GlobeGuts;
                let chunk = globe
                    .chunks()
                    .get(&chunk_origin)
                    .expect("We just ensured this chunk is loaded.");
                // TODO: Gah, such slow!
                chunk.accessible_chunks.clone()
            };
            for accessible_chunk_origin in accessible_chunks {
                globe.ensure_chunk_present(accessible_chunk_origin);

                // Repeat this from each immediately accessible chunk.
                let next_level_accessible_chunks = {
                    use super::globe::GlobeGuts;
                    let chunk = globe
                        .chunks()
                        .get(&accessible_chunk_origin)
                        .expect("We just ensured this chunk is loaded.");
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
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Globe>,
        ReadStorage<'a, CellDweller>,
    );

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
