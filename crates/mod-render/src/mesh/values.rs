use std::mem::size_of;
use bytemuck::cast_slice;

use super::Attribute;



/// 인덱스 데이터입니다.
#[derive(Debug, Clone)]
pub struct Indices(pub Vec<u32>);

impl Indices {
    /// 인덱스 데이터가 비어있는 경우 `true`를 반환합니다.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 인덱스 데이터의 바이트 단위 크기를 반환합니다.
    #[inline]
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 인덱스 데이터의 갯수를 반환합니다.
    #[inline]
    pub fn count(&self) -> usize {
        self.0.len()
    }

    /// 인덱스 데이터의 바이트 단위 크기를 반환합니다.
    #[inline]
    pub fn stride(&self) -> usize {
        size_of::<u32>()
    }

    /// 인덱스 데이터의 바이트 배열을 반환합니다.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        cast_slice(&self.0)
    }
}



/// 정점 데이터입니다.
#[derive(Debug, Clone)]
pub struct Vertices(pub Vec<gmm::Float3>);

impl Vertices {
    /// 정점 데이터가 비어있는 경우 `true`를 반환합니다.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 정점 데이터의 바이트 단위 크기를 반환합니다.
    #[inline]
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 정점 데이터의 갯수를 반환합니다.
    #[inline]
    pub fn count(&self) -> usize {
        self.0.len()
    }

    /// 정점 데이터 요소의 바이트 단위 크기를 반환합니다.
    #[inline]
    pub fn stride(&self) -> usize {
        size_of::<gmm::Float3>()
    }

    /// 정점 데이터의 바이트 배열을 반환합니다.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        cast_slice(&self.0)
    }
}



/// 정점 속성 데이터입니다.
#[derive(Debug, Clone)]
pub enum VertexAttributeValues {
    Colors(Vec<gmm::Float4>), 
    Normals(Vec<gmm::Float3>), 
    Tangents(Vec<gmm::Float3>), 
    Texcoords0(Vec<gmm::Float2>), 
    Texcoords1(Vec<gmm::Float2>), 
    BoneIndices(Vec<gmm::UInteger4>), 
    BoneWeights(Vec<gmm::Float4>), 
}

impl VertexAttributeValues {
    /// 정점 속성 데이터가 비어있는 경우 `true`를 반환합니다.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 정점 속성 데이터의 바이트 단위 크기를 반환합니다.
    #[inline]
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 정점 속성 데이터의 갯수를 반환합니다.
    #[inline]
    pub fn count(&self) -> usize {
        match self {
            VertexAttributeValues::Colors(values) => values.len(), 
            VertexAttributeValues::Normals(values) => values.len(), 
            VertexAttributeValues::Tangents(values) => values.len(), 
            VertexAttributeValues::Texcoords0(values) => values.len(), 
            VertexAttributeValues::Texcoords1(values) => values.len(), 
            VertexAttributeValues::BoneIndices(values) => values.len(), 
            VertexAttributeValues::BoneWeights(values) => values.len(), 
        }
    }

    /// 정점 속성 데이터 요소의 바이트 단위 크기를 반환합니다.
    #[inline]
    pub fn stride(&self) -> usize {
        match self {
            VertexAttributeValues::Colors(_) => size_of::<gmm::Float4>(), 
            VertexAttributeValues::Normals(_) => size_of::<gmm::Float3>(), 
            VertexAttributeValues::Tangents(_) => size_of::<gmm::Float3>(), 
            VertexAttributeValues::Texcoords0(_) => size_of::<gmm::Float2>(), 
            VertexAttributeValues::Texcoords1(_) => size_of::<gmm::Float2>(), 
            VertexAttributeValues::BoneIndices(_) => size_of::<gmm::UInteger4>(), 
            VertexAttributeValues::BoneWeights(_) => size_of::<gmm::Float4>(), 
        }
    }

    /// 정점 속성 데이터의 바이트 배열을 반환합니다.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            VertexAttributeValues::Colors(values) => cast_slice(values), 
            VertexAttributeValues::Normals(values) => cast_slice(values), 
            VertexAttributeValues::Tangents(values) => cast_slice(values), 
            VertexAttributeValues::Texcoords0(values) => cast_slice(values), 
            VertexAttributeValues::Texcoords1(values) => cast_slice(values), 
            VertexAttributeValues::BoneIndices(values) => cast_slice(values), 
            VertexAttributeValues::BoneWeights(values) => cast_slice(values), 
        }
    }

    /// 해당 정점 속성을 반환합니다.
    #[inline]
    pub fn attribute(&self) -> Attribute {
        match self {
            VertexAttributeValues::Colors(_) => Attribute::Colors, 
            VertexAttributeValues::Normals(_) => Attribute::Normals, 
            VertexAttributeValues::Tangents(_) => Attribute::Tangents, 
            VertexAttributeValues::Texcoords0(_) => Attribute::Texcoords0, 
            VertexAttributeValues::Texcoords1(_) => Attribute::Texcoords1, 
            VertexAttributeValues::BoneIndices(_) => Attribute::BoneIndices, 
            VertexAttributeValues::BoneWeights(_) => Attribute::BoneWeights, 
        }
    }
}
