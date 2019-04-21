use std::any;

use froggy;
use gfx;
use slog::Logger;

use super::mesh::Mesh;
use super::Vertex;

// Hide the concrete type of the `Mesh` (specifically the graphics backend)
// by use of `Any`. TODO: there has GOT to be a better way to do this. All I really
// want is to be able to make a `Pointer` to a trait that `Mesh` implements,
// and use that to access the `Storage` of the specific type.
pub struct MeshWrapper {
    mesh: Box<dyn any::Any + Send + Sync + 'static>,
}

/// `gfx::Factory` is not `Send`, so we can't send that around
/// between threads. Instead this mesh repository is shared through
/// an `Arc<Mutex<_>>` and new meshes are only ever created from
/// the main thread, which owns the graphics device and therefore
/// also owns the factory. See `create` below for more.
pub struct MeshRepository<R: gfx::Resources> {
    log: Logger,
    mesh_storage: froggy::Storage<MeshWrapper>,
    default_output_color_buffer: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
    default_output_stencil_buffer: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
}

impl<R: gfx::Resources> MeshRepository<R> {
    pub fn new(
        default_output_color_buffer: gfx::handle::RenderTargetView<R, gfx::format::Srgba8>,
        default_output_stencil_buffer: gfx::handle::DepthStencilView<R, gfx::format::DepthStencil>,
        parent_log: &Logger,
    ) -> MeshRepository<R> {
        MeshRepository {
            mesh_storage: froggy::Storage::new(),
            default_output_color_buffer,
            default_output_stencil_buffer,
            log: parent_log.new(o!()),
        }
    }

    pub fn create<F: gfx::Factory<R>>(
        &mut self,
        factory: &mut F,
        vertexes: Vec<Vertex>,
        triangle_vertex_indexes: Vec<u32>,
    ) -> froggy::Pointer<MeshWrapper> {
        let mesh = Mesh::new(
            factory,
            vertexes,
            triangle_vertex_indexes,
            self.default_output_color_buffer.clone(),
            self.default_output_stencil_buffer.clone(),
        );
        self.add_mesh(mesh)
    }

    pub fn add_mesh(&mut self, mesh: Mesh<R>) -> froggy::Pointer<MeshWrapper> {
        trace!(self.log, "Adding mesh");
        self.mesh_storage.create(MeshWrapper {
            mesh: Box::new(mesh),
        })
    }

    /// Destroy any unused meshes by asking the `froggy::Storage` to catch
    /// up on its internal bookkeeping.
    pub fn collect_garbage(&mut self) {
        self.mesh_storage.sync_pending()
    }
}

impl<'a, R: gfx::Resources> MeshRepository<R> {
    pub fn get_mut(&'a mut self, mesh_pointer: &froggy::Pointer<MeshWrapper>) -> &'a mut Mesh<R> {
        let mesh_wrapper = &mut self.mesh_storage[&mesh_pointer];
        let any_mesh_with_extra_constraints = &mut *mesh_wrapper.mesh;
        let any_mesh = any_mesh_with_extra_constraints as &mut dyn any::Any;
        any_mesh
            .downcast_mut::<Mesh<R>>()
            .expect("Unless we're mixing graphics backends, this should be impossible.")
    }
}
