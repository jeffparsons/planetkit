use na;
use specs;
use specs::{WriteStorage, Read, Write};
use specs::Entities;
use slog::Logger;

use types::*;
use globe::{Globe, View, ChunkView};
use render::{Visual, ProtoMesh, Vertex};
use Spatial;
use nphysics::WorldResource;

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

    fn build_chunk_geometry<'a>(
        &mut self,
        mut globes: specs::WriteStorage<'a, Globe>,
        mut visuals: specs::WriteStorage<'a, Visual>,
        // TODO: Parameterise over ReadStorage/WriteStorage when we don't care?
        // TODO: I made `MaybeMutStorage` for `SpatialStorage`, so just pluck
        // that out somewhere public and use that.
        chunk_views: specs::WriteStorage<'a, ChunkView>,
        world_resource: &mut Write<WorldResource>,
    ) {
        // Throttle rate of geometry creation.
        // We don't want to spend too much doing this.
        let ready = self.seconds_since_last_geometry_creation >
            self.seconds_between_geometry_creation;
        if !ready {
            return;
        }

        use specs::Join;
        for (visual, chunk_view) in (&mut visuals, &chunk_views).join() {
            // TODO: find the closest mesh to the player that needs
            // to be generated (i.e. absent or dirty).
            //
            // TODO: eventually, some rules about capping how many you create.

            // Get the associated globe, complaining loudly if we fail.
            let globe_entity = chunk_view.globe_entity;
            let globe = match globes.get_mut(globe_entity) {
                Some(globe) => globe,
                None => {
                    warn!(
                        self.log,
                        "The globe associated with this ChunkView is not alive! Can't proceed!"
                    );
                    continue;
                }
            };

            // Only re-generate geometry if the chunk is marked as having
            // been changed since last time the view was updated.
            //
            // Note that this will also be true if geometry has never been created for this chunk.
            //
            // TODO: this might also need to be done any time any of its neighboring
            // chunks changes, because we cull invisible cells, and what cells are
            // visible partly depends on what's in neighboring chunks.
            use globe::globe::GlobeGuts;
            let spec = globe.spec();
            {
                // Ew, can I please have non-lexical borrow scopes?
                let chunk = &mut globe.chunks_mut().get(&chunk_view.origin)
                    .expect("Don't know how to deal with chunk not loaded yet. Why do we have a view for it anyway?");
                if !chunk.is_view_dirty {
                    continue;
                }
            }

            // Make a proto-mesh for the chunk.
            // TODO: debounce/throttle per chunk.
            trace!(self.log, "Making chunk proto-mesh"; "origin" => format!("{:?}", chunk_view.origin));
            // TEMP: just use the existing globe `View` struct
            // to get this done. TODO: move into `ChunkView`.
            let globe_view = View::new(spec, &self.log);
            // Build geometry for this chunk into vertex
            // and index buffers.
            let mut vertex_data: Vec<Vertex> = Vec::new();
            let mut index_data: Vec<u32> = Vec::new();
            globe_view.make_chunk_geometry(
                globe,
                chunk_view.origin,
                &mut vertex_data,
                &mut index_data,
            );

            // Mark the chunk as having a clean view.
            // NOTE: we need to do this before maybe skipping
            // actually building the view.
            {
                // Ew, can I please have non-lexical borrow scopes?
                let chunk = &mut globe.chunks_mut().get_mut(&chunk_view.origin)
                    .expect("Don't know how to deal with chunk not loaded yet. Why do we have a view for it anyway?");
                chunk.mark_view_as_clean();
            }

            // Don't attempt to create an empty mesh.
            // Back-end doesn't seem to like this, and there's no point
            // in wasting the VBOs etc. for nothing.
            if vertex_data.is_empty() || index_data.is_empty() {
                trace!(self.log, "Skipping chunk proto-mesh that would be empty"; "origin" => format!("{:?}", chunk_view.origin));

                // TODO: is there anything that will assume we need to make the
                // mesh again just because there's no mesh for the view?
                // Maybe we need to make the case of an empty `Visual` explicit
                // in that type to avoid mistakes.
                continue;
            }

            // Add the terrain mesh to the physics world.
            // TODO: This is a hack. Not here. Need to come up with
            // a better way of doing "reactive" stuff where there
            // are multiple downstream things (visual mesh, physics
            // mesh) derived from a chunk.
            use ncollide3d::shape::{TriMesh, ShapeHandle};
            use nphysics3d::object::{BodyHandle, Material};
            let chunk_origin_pos = globe.spec().cell_bottom_center(*chunk_view.origin.pos());
            let vertices: Vec<Pt3> = vertex_data
                .iter()
                .map(|v| {
                    Pt3::new(
                        v.a_pos[0].into(),
                        v.a_pos[1].into(),
                        v.a_pos[2].into(),
                    )
                })
                .collect();
            let indices: Vec<na::Point3<usize>> = index_data
                .chunks(3)
                .map(|slice| na::Point3::new(slice[0] as usize, slice[1] as usize, slice[2] as usize))
                .collect();
            let tri_mesh = TriMesh::<Real>::new(vertices, indices, None);
            let tri_mesh_handle = ShapeHandle::new(tri_mesh);
            let world = &mut world_resource.world;
            let collision_object_handle = world.add_collider(
                0.01 as Real, // TODO: What's appropriate?
                tri_mesh_handle,
                BodyHandle::ground(),
                Iso3::new(chunk_origin_pos.coords, na::zero()),
                Material::default(),
            );

            visual.proto_mesh = ProtoMesh::new(vertex_data, index_data).into();

            trace!(self.log, "Made chunk proto-mesh"; "origin" => format!("{:?}", chunk_view.origin));

            // Do at most 1 per frame; probably far less.
            self.seconds_since_last_geometry_creation = 0.0;
            return;
        }
    }

    pub fn remove_views_for_dead_chunks<'a>(
        &mut self,
        entities: &Entities<'a>,
        globe: &mut Globe,
        globe_entity: specs::Entity,
        visuals: &mut specs::WriteStorage<'a, Visual>,
        chunk_views: &mut specs::WriteStorage<'a, ChunkView>,
    ) {
        use specs::Join;

        let mut entities_to_remove: Vec<specs::Entity> = Vec::new();

        for (chunk_view, chunk_view_ent) in (&*chunk_views, &**entities).join() {
            // Ignore chunks not belonging to this globe.
            if chunk_view.globe_entity != globe_entity {
                continue;
            }

            if globe.chunk_at(chunk_view.origin).is_none() {
                debug!(self.log, "Removing a chunk view"; "origin" => format!("{:?}", chunk_view.origin));
                entities_to_remove.push(chunk_view_ent);
            }
        }

        for chunk_view_ent in entities_to_remove {
            // TODO: don't forget to remove the MESH.
            // TODO: don't forget to remove the MESH.
            // TODO: don't forget to remove the MESH.
            // TODO: don't forget to remove the MESH.
            // TODO: don't forget to remove the MESH.
            //
            // If you don't do that, then we'll slowly leak VBOs etc.
            //
            // But you can get away with not doing that for now because
            // the tests don't start the render system, and so will never
            // make those meshes to begin with.

            // Remove Visual and ChunkView components (to prevent accidentally
            // iterating over them later within the same frame) and then queue
            // the entity itself up for deletion.
            visuals.remove(chunk_view_ent);
            chunk_views.remove(chunk_view_ent);
            entities.delete(chunk_view_ent).expect("Somehow tried to use an entity with the wrong generation!");

            // TODO: maintain? Do we need to do that here?
            // Or are these entities immediately inaccessible?
            // (I'm unclear on when it's necessary / exactly what it does.)

        }
    }

    pub fn ensure_chunk_view_entities<'a>(
        &mut self,
        entities: &Entities<'a>,
        globe: &mut Globe,
        globe_entity: specs::Entity,
        chunk_views: &mut specs::WriteStorage<'a, ChunkView>,
        visuals: &mut specs::WriteStorage<'a, Visual>,
        spatials: &mut specs::WriteStorage<'a, Spatial>,
    ) {
        use globe::globe::GlobeGuts;
        let globe_spec = globe.spec();
        for chunk in globe.chunks_mut().values_mut() {
            if chunk.view_entity.is_some() {
                continue;
            }
            trace!(self.log, "Making a chunk view"; "origin" => format!("{:?}", chunk.origin));
            let chunk_view = ChunkView::new(globe_entity, chunk.origin);

            // We store the geometry relative to the bottom-center of the chunk origin cell.
            let chunk_origin_pos = globe_spec.cell_bottom_center(*chunk.origin.pos());
            let chunk_transform = Iso3::new(chunk_origin_pos.coords, na::zero());

            // We'll fill it in later.
            let empty_visual = ::render::Visual::new_empty();
            // TODO: Use `create_later_build`, now that it exists?
            // Updated TODO: figure out what the new API is for that.
            // ^^ YES, definitely use that.
            let new_ent = entities.create();
            // TODO: run `maintain`? When do we have to do that?
            // Nope, that is run outside of systems.
            chunk.view_entity = Some(new_ent);
            chunk_views.insert(new_ent, chunk_view)
                .expect("Inserting chunk view failed");
            visuals.insert(new_ent, empty_visual)
                .expect("Inserting empty visual failed");
            spatials.insert(new_ent, Spatial::new(globe_entity, chunk_transform))
                .expect("Inserting spatial failed");
        }
    }
}

impl<'a> specs::System<'a> for ChunkViewSystem {
    type SystemData = (Entities<'a>,
     Read<'a, TimeDeltaResource>,
     WriteStorage<'a, Globe>,
     WriteStorage<'a, Visual>,
     WriteStorage<'a, Spatial>,
     WriteStorage<'a, ChunkView>,
     Write<'a, WorldResource>,
    );

    fn run(&mut self, data: Self::SystemData) {
        use specs::Join;
        let (entities, dt, mut globes, mut visuals, mut spatials, mut chunk_views, mut world_resource) = data;

        self.seconds_since_last_geometry_creation += dt.0;

        // Destroy views for any chunks that are no longer loaded.
        for (globe, globe_entity) in (&mut globes, &*entities).join() {
            self.remove_views_for_dead_chunks(
                &entities,
                globe,
                globe_entity,
                &mut visuals,
                &mut chunk_views,
            );

            // Ensure that there is a visual for
            // every chunk currently loaded in the globe.
            //
            // TODO: we don't actually want to do this
            // long-term; it's just a first step in migrating
            // to systems-based view creation. Eventually we'll
            // be selective about what views to have; i.e. we might
            // have 1000 chunks loaded, and only render 200 of them
            // on this client.
            self.ensure_chunk_view_entities(
                &entities,
                globe,
                globe_entity,
                &mut chunk_views,
                &mut visuals,
                &mut spatials,
            );
        }

        // Build geometry for some chunks; throttled
        // so we don't spend too much time doing this each frame.
        self.build_chunk_geometry(globes, visuals, chunk_views, &mut world_resource);
    }
}
