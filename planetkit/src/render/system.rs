use std::sync::{ Arc, Mutex };
use gfx;
use gfx::Primitive;
use gfx::state::Rasterizer;
use vecmath;
use camera_controllers;
use specs;
use specs::Entities;
use specs::{ ReadStorage, Fetch };
use slog::Logger;

use super::default_pipeline::pipe;
use super::mesh::MeshGuts;
use super::EncoderChannel;
use super::Visual;
use super::MeshRepository;
use ::Spatial;
use ::camera::DefaultCamera;

// System to render all visible entities. This is back-end agnostic;
// i.e. nothing in it should be tied to OpenGL, Vulkan, etc.

pub struct System<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
    _log: Logger,
    // TODO: multiple PSOs
    pso: gfx::PipelineState<R, pipe::Meta>,
    mesh_repo: Arc<Mutex<MeshRepository<R>>>,
    encoder_channel: EncoderChannel<R, C>,
    output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
    output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
    projection: Arc<Mutex<[[f32; 4]; 4]>>,
}

impl<R: gfx::Resources, C: gfx::CommandBuffer<R>> System<R, C> {
    pub fn new<F: gfx::Factory<R>>(
        factory: &mut F,
        encoder_channel: EncoderChannel<R, C>,
        output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
        projection: Arc<Mutex<[[f32; 4]; 4]>>,
        parent_log: &Logger,
        mesh_repo: Arc<Mutex<MeshRepository<R>>>,
    ) -> System<R, C> {
        let log = parent_log.new(o!("system" => "render"));
        debug!(log, "Initialising");

        // Create pipeline state object.
        use gfx::traits::FactoryExt;
        let vs_bytes = include_bytes!("../shaders/copypasta_150.glslv");
        let ps_bytes = include_bytes!("../shaders/copypasta_150.glslf");
        let program = factory.link_program(vs_bytes, ps_bytes).unwrap();
        let pso = factory.create_pipeline_from_program(
            &program,
            Primitive::TriangleList,
            Rasterizer::new_fill().with_cull_back(),
            pipe::new()
        ).unwrap();

        System {
            pso: pso,
            encoder_channel: encoder_channel,
            output_color: output_color,
            output_stencil: output_stencil,
            projection: projection,
            _log: log,
            mesh_repo: mesh_repo,
        }
    }

    // Abstract over `specs` storage types with `A`, and `D`.
    fn draw<'a>(
        &mut self,
        entities: &specs::Entities<'a>,
        visuals: &specs::ReadStorage<'a, Visual>,
        spatials: &specs::ReadStorage<'a, Spatial>,
        camera: specs::Entity,
    ) {
        // TODO: Systems are currently run on the main thread,
        // so we need to `try_recv` to avoid deadlock.
        // This is only because I don't want to burn CPU, and I've yet
        // to get around to frame/update rate limiting, so I'm
        // relying on Piston's for now.
        use std::sync::mpsc::TryRecvError;
        let mut encoder = match self.encoder_channel.receiver.try_recv() {
            Ok(encoder) => encoder,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => panic!("Device owner hung up. That wasn't supposed to happen!"),
        };

        const CLEAR_COLOR: [f32; 4] = [0.3, 0.3, 0.3, 1.0];
        encoder.clear(&self.output_color, CLEAR_COLOR);
        encoder.clear_depth(&self.output_stencil, 1.0);

        let projection = self.projection.lock().unwrap();
        let mut mesh_repo = self.mesh_repo.lock().unwrap();

        // Try to draw all visuals.
        use specs::Join;
        for (entity, visual) in (&**entities, visuals).join() {
            use ::spatial::SpatialStorage;

            // Don't try to draw things that aren't in the same
            // spatial tree as the camera.
            if !spatials.have_common_ancestor(entity, camera) {
                continue;
            }

            // Visual might not have its mesh created yet.
            let mesh_pointer = match visual.mesh_pointer() {
                Some(mesh_pointer) => mesh_pointer,
                None => continue,
            };

            // Transform spatial relative to camera.
            let camera_relative_transform = spatials.a_relative_to_b(
                entity,
                camera,
            );

            // TODO: cache the model matrix separately per Visual
            // if there's a common ancestor that stays the same
            // for a while.
            use na;
            use na::{ Isometry3, Point3, Vector3 };
            let model: Isometry3<f32> = na::convert(camera_relative_transform);

            // Turn the camera's model transform into a view matrix.
            // (This is basically just switching the z-direction because
            // in view space positive z points out of the screen.)
            let view: Isometry3<f32> = Isometry3::look_at_rh(
                &Point3::origin(),
                &Point3::from_coordinates(Vector3::z()),
                &Vector3::y(),
            );

            let model_view = view * model;
            let model_view_matrix = model_view.to_homogeneous();

            // Massage it into a nested array structure and clone it,
            // because `camera_controllers` wants to take ownership.
            let mut model_for_camera_controllers: vecmath::Matrix4<f32> = vecmath::mat4_id();
            // Really? Ew.
            // TODO: probably not getting any value out of `camera_controllers`
            // anymore that you can't get from `nalgebra`.
            model_for_camera_controllers[0].copy_from_slice(&model_view_matrix.as_slice()[0..4]);
            model_for_camera_controllers[1].copy_from_slice(&model_view_matrix.as_slice()[4..8]);
            model_for_camera_controllers[2].copy_from_slice(&model_view_matrix.as_slice()[8..12]);
            model_for_camera_controllers[3].copy_from_slice(&model_view_matrix.as_slice()[12..16]);

            let model_view_projection = camera_controllers::model_view_projection(
                model_for_camera_controllers,
                vecmath::mat4_id(),
                *projection
            );

            let mesh = mesh_repo.get_mut(mesh_pointer);
            mesh.data_mut().u_model_view_proj = model_view_projection;
            encoder.draw(
                mesh.slice(),
                &self.pso,
                mesh.data(),
            );
        }

        self.encoder_channel.sender.send(encoder).unwrap();
    }
}

impl<'a, R, C> specs::System<'a> for System<R, C> where
R: 'static + gfx::Resources,
C: 'static + gfx::CommandBuffer<R> + Send,
{
    type SystemData = (
        Entities<'a>,
        Fetch<'a, DefaultCamera>,
        ReadStorage<'a, Visual>,
        ReadStorage<'a, Spatial>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, default_camera, visuals, spatials) = data;
        self.draw(&entities, &visuals, &spatials, default_camera.camera_entity);

        // TODO: implement own "extrapolated time" concept or similar
        // to decide how often we should actually be trying to render?
        // See https://github.com/PistonDevelopers/piston/issues/193
    }
}
