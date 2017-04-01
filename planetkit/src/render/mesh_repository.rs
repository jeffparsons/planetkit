use gfx;
use slog::Logger;

use super::Vertex;
use super::mesh::Mesh;

#[derive(Copy, Clone, Debug)]
pub struct MeshHandle {
    index: usize,
}

impl MeshHandle {
    fn new(index: usize) -> MeshHandle {
        MeshHandle {
            index: index,
        }
    }
}

/// `gfx::Factory` is not `Send`, so we can't send that around
/// between threads. Instead this mesh repository is shared through
/// an `Arc<Mutex<_>>` and new meshes are only ever created from
/// the main thread, which owns the graphics device and therefore
/// also owns the factory. See `create` below for more.
pub struct MeshRepository<R: gfx::Resources> {
    log: Logger,
    // Meshes can be removed, leaving a gap.
    // TODO: reconsider storage?
    meshes: Vec<Option<Mesh<R>>>,
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
            meshes: Vec::new(),
            default_output_color_buffer: default_output_color_buffer,
            default_output_stencil_buffer: default_output_stencil_buffer,
            log: parent_log.new(o!()),
        }
    }

    pub fn create<F: gfx::Factory<R>>(
        &mut self,
        factory: &mut F,
        vertexes: Vec<Vertex>,
        triangle_vertex_indexes: Vec<u32>,
    ) -> MeshHandle {
        let mesh = Mesh::new(
            factory,
            vertexes,
            triangle_vertex_indexes,
            self.default_output_color_buffer.clone(),
            self.default_output_stencil_buffer.clone(),
        );
        self.add_mesh(mesh)
    }

    pub fn add_mesh(&mut self, mesh: Mesh<R>) -> MeshHandle {
        trace!(self.log, "Adding mesh");
        self.meshes.push(mesh.into());
        MeshHandle::new(self.meshes.len() - 1)
    }

    pub fn replace_mesh(&mut self, mesh_handle: MeshHandle, mesh: Mesh<R>) {
        trace!(self.log, "Replacing mesh {}", format!("{:?}", mesh_handle));
        self.meshes[mesh_handle.index] = mesh.into();
    }
}

impl<'a, R: gfx::Resources> MeshRepository<R> {
    pub fn get_mut(&'a mut self, mesh_handle: MeshHandle) -> Option<&'a mut Mesh<R>>{
        self.meshes[mesh_handle.index].as_mut()
    }
}
