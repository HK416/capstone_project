use gmm::{
    Float2, Float3, Float4, 
    Integer2, Integer3, Integer4, 
    UInteger2, UInteger3, UInteger4, 
};



/// 정점 속성의 값 입니다.
#[derive(Debug, Clone, Copy)]
pub enum VertexAttributeValues<'a> {
    Float(&'a [f32]), 
    Float2(&'a [Float2]), 
    Float3(&'a [Float3]), 
    Float4(&'a [Float4]), 
    Int(&'a [i32]), 
    Int2(&'a [Integer2]), 
    Int3(&'a [Integer3]), 
    Int4(&'a [Integer4]), 
    UInt(&'a [u32]), 
    UInt2(&'a [UInteger2]), 
    Uint3(&'a [UInteger3]), 
    UInt4(&'a [UInteger4]), 
}

impl<'a> VertexAttributeValues<'a> {
    /// 정점 속성의 바이트 단위 크기를 반환합니다.
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 정점 속성 요소의 크기를 반환합니다.
    pub fn stride(&self) -> usize {
        use std::mem;
        match self {
            VertexAttributeValues::Float(_) => mem::size_of::<f32>(),
            VertexAttributeValues::Float2(_) => mem::size_of::<Float2>(),
            VertexAttributeValues::Float3(_) => mem::size_of::<Float3>(),
            VertexAttributeValues::Float4(_) => mem::size_of::<Float4>(),
            VertexAttributeValues::Int(_) => mem::size_of::<i32>(),
            VertexAttributeValues::Int2(_) => mem::size_of::<Integer2>(),
            VertexAttributeValues::Int3(_) => mem::size_of::<Integer3>(),
            VertexAttributeValues::Int4(_) => mem::size_of::<Integer4>(),
            VertexAttributeValues::UInt(_) => mem::size_of::<u32>(),
            VertexAttributeValues::UInt2(_) => mem::size_of::<UInteger2>(),
            VertexAttributeValues::Uint3(_) => mem::size_of::<UInteger3>(),
            VertexAttributeValues::UInt4(_) => mem::size_of::<UInteger4>(),
        }
    }

    /// 정점 속성 요소의 갯수를 반환합니다.
    pub fn count(&self) -> usize {
        match self {
            VertexAttributeValues::Float(it) => it.len(),
            VertexAttributeValues::Float2(it) => it.len(),
            VertexAttributeValues::Float3(it) => it.len(),
            VertexAttributeValues::Float4(it) => it.len(),
            VertexAttributeValues::Int(it) => it.len(),
            VertexAttributeValues::Int2(it) => it.len(),
            VertexAttributeValues::Int3(it) => it.len(),
            VertexAttributeValues::Int4(it) => it.len(),
            VertexAttributeValues::UInt(it) => it.len(),
            VertexAttributeValues::UInt2(it) => it.len(),
            VertexAttributeValues::Uint3(it) => it.len(),
            VertexAttributeValues::UInt4(it) => it.len(), 
        }
    }

    /// 바이트 배열을 반환합니다.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            VertexAttributeValues::Float(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::Float2(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::Float3(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::Float4(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::Int(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::Int2(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::Int3(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::Int4(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::UInt(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::UInt2(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::Uint3(it) => bytemuck::cast_slice(it),
            VertexAttributeValues::UInt4(it) => bytemuck::cast_slice(it),
        }
    }
}
