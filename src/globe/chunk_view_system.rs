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
        Gd: Deref<Target = specs::MaskedStorage<Globe>>,
        Vd: DerefMut<Target = specs::MaskedStorage<Visual>>,
        Cd: Deref<Target = specs::MaskedStorage<ChunkView>>,
    >(
        &mut self,
        globes: specs::Storage<Globe, A, Gd>,
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
            if visual.mesh_handle().is_some() ||
                visual.proto_mesh.is_some() {
                // There's already a visual for this mesh.
                // TODO: consider replacing it if it's dirty.
                continue;
            }

            trace!(self.log, "Making chunk proto-mesh"; "origin" => format!("{:?}", chunk_view.origin));

            // Make a proto-mesh for the chunk.
            // Get the associated globe, complaining loudly if we fail.
            let globe_entity = chunk_view.globe_entity;
            let globe = match globes.get(globe_entity) {
                Some(globe) => globe,
                None => {
                    warn!(self.log, "The globe associated with this ChunkView is not alive! Can't proceed!");
                    continue;
                },
            };
            // TEMP: just use the existing globe `View` struct
            // to get this done. TODO: move into `ChunkView`.
            let globe_view = View::new(
                globe,
                &self.log,
            );
            // Build geometry for this chunk into vertex
            // and index buffers.
            let chunk_index = globe.index_of_chunk_at(chunk_view.origin).expect("Don't know how to deal with chunk not loaded yet. Why do we have a view for it anyway?");
            use globe::globe::GlobeGuts;
            let chunk = &globe.chunks()[chunk_index];
            let mut vertex_data: Vec<Vertex> = Vec::new();
            let mut index_data: Vec<u32> = Vec::new();
            globe_view.make_chunk_geometry(
                chunk,
                &mut vertex_data,
                &mut index_data,
            );
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
