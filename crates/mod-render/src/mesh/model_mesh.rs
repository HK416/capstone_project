use std::collections::HashMap;

use super::{Attribute, IndexBuffer, VertexBuffer};



/// 같은 메쉬를 사용하는 서로 다른 오브젝트간에 공유할 수 있는 메쉬 데이터입니다.
#[derive(Debug)]
pub struct ModelMesh {
    /// 메쉬의 이름입니다.
    pub(crate) name: String, 

    /// 메쉬의 정점 개수입니다.
    pub(crate) num_vertices: u32, 

    /// 메쉬의 정점 버퍼입니다.
    pub(crate) vertex: VertexBuffer, 

    /// 메쉬의 정점 속성 버퍼입니다.
    pub(crate) attributes: HashMap<Attribute, VertexBuffer>, 

    /// 메쉬의 하위 메쉬들입니다.
    pub(crate) submeshes: Vec<IndexBuffer>, 
}

impl ModelMesh {
    /// 메쉬의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 정점의 개수를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn num_vertices(&self) -> u32 {
        self.num_vertices
    }

    /// 메쉬의 정점 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn vertex(&self) -> &VertexBuffer {
        &self.vertex
    }

    /// 메쉬의 정점 속성 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn attribute(&self, id: &Attribute) -> Option<&VertexBuffer> {
        self.attributes.get(id)
    }

    /// 메쉬의 하위 메쉬들을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn submeshes(&self) -> &[IndexBuffer] {
        &self.submeshes
    }
}
