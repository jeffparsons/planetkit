use std::ops::{ Deref, DerefMut };

use specs;
use slog::Logger;

use types::*;
use globe::{ Globe, View, ChunkView };
use ::render::{ Visual, ProtoMesh, Vertex };

// For now, just creates up to 1 chunk view per tick,
// until we have created views for all chunks.
pub struct ChunkViewSystem {
    log: Logger,
    seconds_between_geometry_creation: TimeDelta,
    seconds_since_last_geometry_creation: TimeDelta,
}

impl ChunkViewSystem {
    pub fn new(
        parent_log: &Logger,
        seconds_between_geometry_creation: TimeDelta,
    ) -> ChunkViewSystem {
        ChunkViewSystem {
            log: parent_log.new(o!()),
            seconds_between_geometry_creation: seconds_between_geometry_creation,
            seconds_since_last_geometry_creation: 0.0,
        }
    }

    fn build_chunk_geometry<
        A: Deref<Target = specs::Allocator>,
        Gd: DerefMut<Target = specs::MaskedStorage<Globe>>,
        Vd: DerefMut<Target = specs::MaskedStorage<Visual>>,
        Cd: Deref<Target = specs::MaskedStorage<ChunkView>>,
    >(
        &mut self,
        mut globes: specs::Storage<Globe, A, Gd>,
        mut visuals: specs::Storage<Visual, A, Vd>,
        chunk_views: specs::Storage<ChunkView, A, Cd>,
    ) {
        // Throttle rate of geometry creation.
        // We don't want to spend too much doing this.
        let ready = self.seconds_since_last_geometry_creation > self.seconds_between_geometry_creation;
        if !ready {
            return;
        }

        use specs::Join;
        for (visual, chunk_view) in (&mut visuals, &chunk_views).iter() {
            // TODO: find the closest mesh to the player that needs
            // to be generated (i.e. absent or dirty).
            //
            // TODO: eventually, some rules about capping how many you create.

            // Get the associated globe, complaining loudly if we fail.
            let globe_entity = chunk_view.globe_entity;
            let mut globe = match globes.get_mut(globe_entity) {
                Some(globe) => globe,
                None => {
                    warn!(self.log, "The globe associated with this ChunkView is not alive! Can't proceed!");
                    continue;
                },
            };

            // Only re-generate geometry if the chunk is marked as having
            // been changed since last time the view was updated.
            //
            // Note that this will also be true if geometry has never been created for this chunk.
            let chunk_index = globe.index_of_chunk_at(chunk_view.origin).expect("Don't know how to deal with chunk not loaded yet. Why do we have a view for it anyway?");
            use globe::globe::GlobeGuts;
            let spec = globe.spec();
            let mut chunk = &mut globe.chunks_mut()[chunk_index];
            if !chunk.is_view_dirty {
                continue;
            }

            // Make a proto-mesh for the chunk.
            trace!(self.log, "Making chunk proto-mesh"; "origin" => format!("{:?}", chunk_view.origin));
            // TEMP: just use the existing globe `View` struct
            // to get this done. TODO: move into `ChunkView`.
            let globe_view = View::new(
                spec,
                &self.log,
            );
            // Build geometry for this chunk into vertex
            // and index buffers.
            let mut vertex_data: Vec<Vertex> = Vec::new();
            let mut index_data: Vec<u32> = Vec::new();
            globe_view.make_chunk_geometry(
                chunk,
                &mut vertex_data,
                &mut index_data,
            );

            // Mark the chunk as having a clean view.
            // NOTE: we need to do this before maybe skipping
            // actually building the view.
            chunk.mark_view_as_clean();

            // Don't attempt to create an empty mesh.
            // Back-end doesn't seem to like this, and there's no point
            // in wasting the VBOs etc. for nothing.
            if vertex_data.len() == 0 || index_data.len() == 0 {
                debug!(self.log, "Skipping chunk proto-mesh that would be empty"; "origin" => format!("{:?}", chunk_view.origin));

                // TODO: is there anything that will assume we need to make the
                // mesh again just because there's no mesh for the view?
                // Maybe we need to make the case of an empty `Visual` explicit
                // in that type to avoid mistakes.
                continue;
            }

            visual.proto_mesh = ProtoMesh::new(vertex_data, index_data).into();

            trace!(self.log, "Made chunk proto-mesh"; "origin" => format!("{:?}", chunk_view.origin));

            // Do at most 1 per frame; probably far less.
            self.seconds_since_last_geometry_creation = 0.0;
            return;
        }
    }
}

impl specs::System<TimeDelta> for ChunkViewSystem {
    fn run(&mut self, arg: specs::RunArg, dt: TimeDelta) {
        self.seconds_since_last_geometry_creation += dt;

        use specs::Join;
        let (globes, visuals, chunk_views) = arg.fetch(|w| {
            let mut globes = w.write::<Globe>();
            let entities = w.entities();
            for (globe, globe_entity) in (&mut globes, &entities).iter() {
                // Ensure that there is a visual for
                // every chunk in the globe.
                //
                // TODO: we don't actually want to do this
                // long-term; it's just a first step in migrating
                // to systems-based view creation. Eventually we'll
                // be selective about what views to have.
                globe.ensure_chunk_view_entities(w, globe_entity);
            }
            (globes, w.write::<Visual>(), w.write::<ChunkView>())
        });

        // Build geometry for some chunks; throttled
        // so we don't spend too much time doing this each frame.
        self.build_chunk_geometry(
            globes,
            visuals,
            chunk_views,
        );
    }
}
