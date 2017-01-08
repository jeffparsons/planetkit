use std::ops::Deref;
use std::sync::{ Arc, Mutex };
use gfx;
use gfx::Primitive;
use gfx::state::Rasterizer;
use vecmath;
use camera_controllers;
use specs;
use slog::Logger;

use super::default_pipeline::pipe;
use super::Vertex;
use super::mesh::{ Mesh, MeshGuts };
use super::EncoderChannel;
use super::Visual;
use ::Spatial;
use ::types::*;

// TODO: make this opaque and handle mesh removal.
pub type MeshHandle = usize;

// System to render all visible entities. This is back-end agnostic;
// i.e. nothing in it should be tied to OpenGL, Vulkan, etc.

pub struct System<R: gfx::Resources, C: gfx::CommandBuffer<R>> {
    log: Logger,
    // TODO: multiple PSOs
    pso: gfx::PipelineState<R, pipe::Meta>,
    // OMG the hacks hurt.
    // TODO: at very least make a mesh repository
    // that can be shared.
    pub meshes: Arc<Mutex<Vec<Mesh<R>>>>,
    encoder_channel: EncoderChannel<R, C>,
    output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
    output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
    first_person: Arc<Mutex<camera_controllers::FirstPerson>>,
    projection: Arc<Mutex<[[f32; 4]; 4]>>,
}

impl<R: gfx::Resources, C: gfx::CommandBuffer<R>> System<R, C> {
    pub fn new<F: gfx::Factory<R>>(
        factory: &mut F,
        encoder_channel: EncoderChannel<R, C>,
        output_color: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        output_stencil: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
        first_person: Arc<Mutex<camera_controllers::FirstPerson>>,
        projection: Arc<Mutex<[[f32; 4]; 4]>>,
        parent_log: &Logger,
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
            meshes: Arc::new(Mutex::new(Vec::new())),
            encoder_channel: encoder_channel,
            output_color: output_color,
            output_stencil: output_stencil,
            first_person: first_person,
            projection: projection,
            log: log,
        }
    }

    pub fn create_mesh<F: gfx::Factory<R>>(
        &mut self,
        // TODO: store factory on render system? Need to wrap it in an Arc.
        factory: &mut F,
        vertices: Vec<Vertex>,
        vertex_indices: Vec<u32>,
    ) -> MeshHandle {
        let mesh = Mesh::new(
            factory,
            vertices,
            vertex_indices,
            self.output_color.clone(),
            self.output_stencil.clone(),
        );
        self.add_mesh(mesh)
    }

    pub fn add_mesh(&mut self, mesh: Mesh<R>) -> MeshHandle {
        trace!(self.log, "Adding mesh");
        let mut meshes = self.meshes.lock().unwrap();
        meshes.push(mesh);
        meshes.len() - 1
    }

    // Abstract over `specs` storage types with `A`, and `D`.
    fn draw<
        A: Deref<Target = specs::Allocator>,
        Vd: Deref<Target = specs::MaskedStorage<Visual>>,
        Sd: Deref<Target = specs::MaskedStorage<Spatial>>,
    >(
        &mut self,
        dt: TimeDelta,
        visuals: specs::Storage<Visual, A, Vd>,
        spatials: specs::Storage<Spatial, A, Sd>,
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

        let fp = self.first_person.lock().unwrap();
        let projection = self.projection.lock().unwrap();
        let mut meshes = self.meshes.lock().unwrap();

        // Try to draw all visuals.
        use specs::Join;
        for (v, s) in (&visuals, &spatials).iter() {
            // Visual might not have its mesh created yet.
            if let Some(mesh_handle) = v.mesh_handle() {
                // TODO: cache the model matrix separately per Visual
                use na;
                use na::{ Vector3, Matrix3, Rotation3, Isometry3, ToHomogeneous };
                // Do some nasty fiddling to cast down to `f32`.
                let transform_f32: Isometry3<f32> = {
                    let translation_f32: Vector3<f32> = na::Cast::<Vector3<f64>>::from(s.transform.translation);
                    let rot_mat_f32: Matrix3<f32> = na::Cast::<Matrix3<f64>>::from(*s.transform.rotation.submatrix());
                    let rotation_f32 = Rotation3::from_matrix_unchecked(rot_mat_f32);
                    Isometry3::from_rotation_matrix(translation_f32, rotation_f32)
                };
                let model = transform_f32.to_homogeneous();
                // Massage it into a nested array structure and clone it,
                // because `camera_controllers` wants to take ownership.
                let mut model_for_camera_controllers: vecmath::Matrix4<f32> = vecmath::mat4_id();
                model_for_camera_controllers.copy_from_slice(model.as_ref());

                let model_view_projection = camera_controllers::model_view_projection(
                    model_for_camera_controllers,
                    fp.camera(dt).orthogonal(),
                    *projection
                );

                let mesh = &mut meshes[mesh_handle];
                mesh.data_mut().u_model_view_proj = model_view_projection;
                encoder.draw(
                    mesh.slice(),
                    &self.pso,
                    mesh.data(),
                );
            }
        }

        self.encoder_channel.sender.send(encoder).unwrap();
    }
}

impl<R, C> specs::System<TimeDelta> for System<R, C> where
R: 'static + gfx::Resources,
C: 'static + gfx::CommandBuffer<R> + Send,
{
    fn run(&mut self, arg: specs::RunArg, dt: TimeDelta) {
        let (visuals, spatials) = arg.fetch(|w|
            (w.read::<Visual>(), w.read::<Spatial>()),
        );

        // TODO: turn any proto-meshes into real meshes.
        // ...but we need access to the resource factory for that.

        self.draw(dt, visuals, spatials);

        // TODO: implement own "extrapolated time" concept or similar
        // to decide how often we should actually be trying to render?
        // See https://github.com/PistonDevelopers/piston/issues/193
    }
}
