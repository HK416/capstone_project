mod buffer;
mod builder;
mod non_skinned_mesh;
mod skinned_mesh;
mod uniform;
mod values;

use std::sync::Arc;

pub use self::buffer::*;
pub use self::builder::*;
pub use self::non_skinned_mesh::*;
pub use self::skinned_mesh::*;
pub use self::uniform::*;
pub use self::values::*;



/// 메쉬 데이터입니다.
#[derive(Debug, Clone)]
pub enum Mesh {
    NonSkinnedMesh(Arc<NonSkinnedMesh>), 
    SkinnedMesh(Arc<SkinnedMesh>)
}

impl Mesh {
    /// 메쉬의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Mesh::NonSkinnedMesh(mesh) => mesh.name(), 
            Mesh::SkinnedMesh(skinned_mesh) => skinned_mesh.name()
        }
    }

    /// 정점의 개수를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn num_vertices(&self) -> u32 {
        match self {
            Mesh::NonSkinnedMesh(mesh) => mesh.num_vertices(), 
            Mesh::SkinnedMesh(skinned_mesh) => skinned_mesh.num_vertices()
        }
    }

    /// 메쉬의 정점 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn vertex(&self) -> &VertexBuffer {
        match self {
            Mesh::NonSkinnedMesh(mesh) => mesh.vertex(), 
            Mesh::SkinnedMesh(skinned_mesh) => skinned_mesh.vertex()
        }
    }

    /// 메쉬의 정점 속성 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn attribute(&self, id: &Attribute) -> Option<&VertexBuffer> {
        match self {
            Mesh::NonSkinnedMesh(mesh) => mesh.attribute(id), 
            Mesh::SkinnedMesh(skinned_mesh) => skinned_mesh.attribute(id)
        }
    }

    /// 메쉬의 하위 메쉬들을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn submeshes(&self) -> &[IndexBuffer] {
        match self {
            Mesh::NonSkinnedMesh(mesh) => mesh.submeshes(), 
            Mesh::SkinnedMesh(skinned_mesh) => skinned_mesh.submeshes()
        }
    }

    /// 메쉬의 바인드 그룹을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        match self {
            Mesh::NonSkinnedMesh(mesh) => mesh.bind_group(), 
            Mesh::SkinnedMesh(skinned_mesh) => skinned_mesh.bind_group()
        }
    }
}
