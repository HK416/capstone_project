mod buffer;
mod builder;
mod model_mesh;
mod non_skinned_mesh;
mod skinned_mesh;
mod uniform;
mod values;

use std::sync::Arc;

pub use self::buffer::*;
pub use self::builder::*;
pub use self::model_mesh::*;
pub use self::non_skinned_mesh::*;
pub use self::skinned_mesh::*;
pub use self::uniform::*;
pub use self::values::*;



/// 메쉬 렌더링 데이터입니다.
#[derive(Debug, Clone)]
pub enum MeshRenderer {
    NonSkinnedMesh(Arc<NonSkinnedMesh>), 
    SkinnedMesh(Arc<SkinnedMesh>)
}

impl MeshRenderer {
    /// 메쉬의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            MeshRenderer::NonSkinnedMesh(mesh) => mesh.model_mesh().name(), 
            MeshRenderer::SkinnedMesh(skinned_mesh) => skinned_mesh.model_mesh().name()
        }
    }

    /// 정점의 개수를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn num_vertices(&self) -> u32 {
        match self {
            MeshRenderer::NonSkinnedMesh(mesh) => mesh.model_mesh().num_vertices(), 
            MeshRenderer::SkinnedMesh(skinned_mesh) => skinned_mesh.model_mesh().num_vertices()
        }
    }

    /// 메쉬의 정점 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn vertex(&self) -> &VertexBuffer {
        match self {
            MeshRenderer::NonSkinnedMesh(mesh) => mesh.model_mesh().vertex(), 
            MeshRenderer::SkinnedMesh(skinned_mesh) => skinned_mesh.model_mesh().vertex()
        }
    }

    /// 메쉬의 정점 속성 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn attribute(&self, id: &Attribute) -> Option<&VertexBuffer> {
        match self {
            MeshRenderer::NonSkinnedMesh(mesh) => mesh.model_mesh().attribute(id), 
            MeshRenderer::SkinnedMesh(skinned_mesh) => skinned_mesh.model_mesh().attribute(id)
        }
    }

    /// 메쉬의 하위 메쉬들을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn submeshes(&self) -> &[IndexBuffer] {
        match self {
            MeshRenderer::NonSkinnedMesh(mesh) => mesh.model_mesh().submeshes(), 
            MeshRenderer::SkinnedMesh(skinned_mesh) => skinned_mesh.model_mesh().submeshes()
        }
    }

    /// 메쉬의 바인드 그룹을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        match self {
            MeshRenderer::NonSkinnedMesh(mesh) => mesh.bind_group(), 
            MeshRenderer::SkinnedMesh(skinned_mesh) => skinned_mesh.bind_group()
        }
    }
}
