use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    ops::RangeBounds,
};

/// ## Vertex Input Attribute Kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AttributeKind {
    Color,
    Normal,
    Tangent,
    Texcoord0,
    Texcoord1,
    Texcoord2,
    Texcoord3,
    BoneIndex,
    BoneWeight,
    TransformColumn0,
    TransformColumn1,
    TransformColumn2,
    TransformColumn3,
}

/// ## Index Buffer Data
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Indices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl Indices {
    /// 인덱스 데이터가 비어있는 경우 `true`를 반환합니다.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 인덱스 데이터 요소의 개수를 반환합니다.
    pub fn count(&self) -> usize {
        match self {
            Indices::U16(v) => v.len(),
            Indices::U32(v) => v.len(),
        }
    }

    /// 인덱스 데이터의 바이트 단위 크기를 반환합니다.
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 인덱스 데이터 요소의 바이트 단위 크기르 반환합니다.
    pub fn stride(&self) -> usize {
        match self {
            Indices::U16(_) => core::mem::size_of::<u16>(),
            Indices::U32(_) => core::mem::size_of::<u32>(),
        }
    }

    /// 인덱스 데이터의 인덱스 포맷을 반환합니다.
    pub fn format(&self) -> wgpu::IndexFormat {
        match self {
            Indices::U16(_) => wgpu::IndexFormat::Uint16,
            Indices::U32(_) => wgpu::IndexFormat::Uint32,
        }
    }

    /// 인덱스 데이터의 바이트 배열을 반환합니다.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Indices::U16(v) => bytemuck::cast_slice(v),
            Indices::U32(v) => bytemuck::cast_slice(v),
        }
    }
}

/// ## Index Buffer
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct IndexBuffer {
    count: u32,
    format: wgpu::IndexFormat,
    buffer: wgpu::Buffer,
}

impl IndexBuffer {
    /// 새로운 인덱스 버퍼를 생성합니다.
    ///
    /// # Debug
    /// 주어진 인덱스 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: Indices,
    ) -> Self {
        debug_assert!(!data.is_empty(), "the given index buffer data is empty");
        unsafe { Self::new_unchecked(label, device, queue, data) }
    }

    /// 새로운 인덱스 버퍼를 생성합니다.
    ///
    /// # Safety
    /// 주어진 인덱스 버퍼 데이터가 비어있지 않는 경우 이 함수는 안전합니다.
    ///
    pub unsafe fn new_unchecked(
        label: Option<&str>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: Indices,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: data.size() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        queue.write_buffer(&buffer, 0, data.as_bytes());

        Self {
            count: data.count() as u32,
            format: data.format(),
            buffer,
        }
    }

    /// 인덱스 버퍼 요소의 개수를 반환합니다.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// 인덱스 버퍼의 인덱스 포맷을 반환합니다.
    pub fn format(&self) -> wgpu::IndexFormat {
        self.format
    }

    /// 범위에 해당하는 슬라이스된 인덱스 버퍼를 반환합니다.
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice<'_>
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.buffer.slice(bounds)
    }
}

/// ## Vertex Buffer Data
#[derive(Debug, Clone, PartialEq)]
pub struct Vertices(pub Vec<[f32; 3]>);

impl Vertices {
    /// 정점 데이터가 비어있는 경우 `true`를 반환합니다.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 정점 데이터 요소의 개수를 반환합니다.
    pub fn count(&self) -> usize {
        self.0.len()
    }

    /// 정점 데이터의 바이트 단위 크기를 반환합니다.
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 정점 데이터 요소의 바이트 단위 크기를 반환합니다.
    pub fn stride(&self) -> usize {
        core::mem::size_of::<f32>() * 3
    }

    /// 정점 데이터의 바이트 배열을 반환합니다.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.0)
    }
}

/// ## Vertex Attribute Buffer Data
#[derive(Debug, Clone, PartialEq)]
pub enum Attributes {
    Color(Vec<[f32; 4]>),
    Normal(Vec<[f32; 3]>),
    Tangent(Vec<[f32; 3]>),
    Texcoord0(Vec<[f32; 2]>),
    Texcoord1(Vec<[f32; 2]>),
    Texcoord2(Vec<[f32; 2]>),
    Texcoord3(Vec<[f32; 2]>),
    BoneIndex(Vec<[u32; 4]>),
    BoneWeight(Vec<[f32; 4]>),
    TransformColumn0(Vec<[f32; 4]>),
    TransformColumn1(Vec<[f32; 4]>),
    TransformColumn2(Vec<[f32; 4]>),
    TransformColumn3(Vec<[f32; 4]>),
}

impl Attributes {
    /// 정점 속성 데이터가 비어있는 경우 `true`를 반환합니다.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 정점 속성 데이터 요소의 개수를 반환합니다.
    pub fn count(&self) -> usize {
        match self {
            Attributes::Color(v) => v.len(),
            Attributes::Normal(v) => v.len(),
            Attributes::Tangent(v) => v.len(),
            Attributes::Texcoord0(v) => v.len(),
            Attributes::Texcoord1(v) => v.len(),
            Attributes::Texcoord2(v) => v.len(),
            Attributes::Texcoord3(v) => v.len(),
            Attributes::BoneIndex(v) => v.len(),
            Attributes::BoneWeight(v) => v.len(),
            Attributes::TransformColumn0(v) => v.len(),
            Attributes::TransformColumn1(v) => v.len(),
            Attributes::TransformColumn2(v) => v.len(),
            Attributes::TransformColumn3(v) => v.len(),
        }
    }

    /// 정점 속성 데이터의 바이트 단위 크기를 반환합니다.
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 정점 속성 데이터 요소의 바이트 단위 크기를 반환합니다.
    pub fn stride(&self) -> usize {
        match self {
            Attributes::Color(_) => core::mem::size_of::<f32>() * 4,
            Attributes::Normal(_) => core::mem::size_of::<f32>() * 3,
            Attributes::Tangent(_) => core::mem::size_of::<f32>() * 3,
            Attributes::Texcoord0(_) => core::mem::size_of::<f32>() * 2,
            Attributes::Texcoord1(_) => core::mem::size_of::<f32>() * 2,
            Attributes::Texcoord2(_) => core::mem::size_of::<f32>() * 2,
            Attributes::Texcoord3(_) => core::mem::size_of::<f32>() * 2,
            Attributes::BoneIndex(_) => core::mem::size_of::<u32>() * 4,
            Attributes::BoneWeight(_) => core::mem::size_of::<f32>() * 4,
            Attributes::TransformColumn0(_) => core::mem::size_of::<f32>() * 4,
            Attributes::TransformColumn1(_) => core::mem::size_of::<f32>() * 4,
            Attributes::TransformColumn2(_) => core::mem::size_of::<f32>() * 4,
            Attributes::TransformColumn3(_) => core::mem::size_of::<f32>() * 4,
        }
    }
    /// 정점 속성 데이터 종류를 반환합니다.
    pub fn kind(&self) -> AttributeKind {
        match self {
            Attributes::Color(_) => AttributeKind::Color,
            Attributes::Normal(_) => AttributeKind::Normal,
            Attributes::Tangent(_) => AttributeKind::Tangent,
            Attributes::Texcoord0(_) => AttributeKind::Texcoord0,
            Attributes::Texcoord1(_) => AttributeKind::Texcoord1,
            Attributes::Texcoord2(_) => AttributeKind::Texcoord2,
            Attributes::Texcoord3(_) => AttributeKind::Texcoord3,
            Attributes::BoneIndex(_) => AttributeKind::BoneIndex,
            Attributes::BoneWeight(_) => AttributeKind::BoneWeight,
            Attributes::TransformColumn0(_) => AttributeKind::TransformColumn0,
            Attributes::TransformColumn1(_) => AttributeKind::TransformColumn1,
            Attributes::TransformColumn2(_) => AttributeKind::TransformColumn2,
            Attributes::TransformColumn3(_) => AttributeKind::TransformColumn3,
        }
    }

    /// 정점 속성 데이터의 바이트 배열을 반환합니다.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Attributes::Color(v) => bytemuck::cast_slice(&v),
            Attributes::Normal(v) => bytemuck::cast_slice(&v),
            Attributes::Tangent(v) => bytemuck::cast_slice(&v),
            Attributes::Texcoord0(v) => bytemuck::cast_slice(&v),
            Attributes::Texcoord1(v) => bytemuck::cast_slice(&v),
            Attributes::Texcoord2(v) => bytemuck::cast_slice(&v),
            Attributes::Texcoord3(v) => bytemuck::cast_slice(&v),
            Attributes::BoneIndex(v) => bytemuck::cast_slice(&v),
            Attributes::BoneWeight(v) => bytemuck::cast_slice(&v),
            Attributes::TransformColumn0(v) => bytemuck::cast_slice(&v),
            Attributes::TransformColumn1(v) => bytemuck::cast_slice(&v),
            Attributes::TransformColumn2(v) => bytemuck::cast_slice(&v),
            Attributes::TransformColumn3(v) => bytemuck::cast_slice(&v),
        }
    }
}

/// ## Vertex Buffer
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct VertexBuffer(wgpu::Buffer);

impl VertexBuffer {
    /// 새로운 정점 버퍼를 생성합니다.
    ///
    /// # Debug
    /// 주어진 정점 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_vertices(
        label: Option<&str>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: Vertices,
    ) -> Self {
        debug_assert!(!data.is_empty(), "the given vertex data is empty");
        unsafe { Self::from_vertices_unchecked(label, device, queue, data) }
    }

    /// 새로운 정점 버퍼를 생성합니다.
    ///
    /// # Safety
    /// 주어진 정점 버퍼 데이터가 비어있지 않는 경우 이 함수는 안전합니다.
    ///
    pub unsafe fn from_vertices_unchecked(
        label: Option<&str>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: Vertices,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: data.size() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        queue.write_buffer(&buffer, 0, data.as_bytes());

        Self(buffer)
    }

    /// 새로운 정점 속성 버퍼를 생성합니다.
    ///
    /// # Debug
    /// 주어진 정점 속성 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_attribute(
        label: Option<&str>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: Attributes,
    ) -> Self {
        debug_assert!(!data.is_empty(), "the given vertex attribute data is empty");
        unsafe { Self::from_attribute_unchecked(label, device, queue, data) }
    }

    /// 새로운 정점 속성 버퍼를 생성합니다.
    ///
    /// # Safety
    /// 주어진 정점 속성 버퍼 데이터가 비어있지 않는 경우 이 함수는 안전합니다.
    ///
    pub unsafe fn from_attribute_unchecked(
        label: Option<&str>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: Attributes,
    ) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            mapped_at_creation: false,
            size: data.size() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        queue.write_buffer(&buffer, 0, data.as_bytes());

        Self(buffer)
    }

    /// 범위에 해당하는 슬라이스된 정점 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice<'_>
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.0.slice(bounds)
    }
}

/// ## Model Mesh
#[derive(Debug, PartialEq, Eq)]
pub struct Mesh {
    name: String,
    num_vertices: u32,
    vertex: VertexBuffer,
    attributes: HashMap<AttributeKind, VertexBuffer>,
    submeshes: Vec<IndexBuffer>,
}

impl Mesh {
    /// 새로운 메쉬를 생성합니다.
    ///
    /// # Debug
    /// 주어진 정점 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(name: &str, device: &wgpu::Device, queue: &wgpu::Queue, data: Vertices) -> Self {
        Self {
            name: name.into(),
            num_vertices: data.count() as u32,
            vertex: VertexBuffer::from_vertices(
                Some(&format!("Vertex({})", &name)),
                device,
                queue,
                data,
            ),
            attributes: HashMap::new(),
            submeshes: Vec::new(),
        }
    }

    /// 정점 속성 버퍼를 추가합니다.  
    /// 이미 해당 정점 속성 버퍼가 존재하는 경우 새로운 정점 속성 버퍼로 교체됩니다.
    ///
    /// # Debug
    /// 주어진 정점 속성 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn add_attribute(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: Attributes,
    ) -> Option<VertexBuffer> {
        self.attributes.insert(
            data.kind(),
            VertexBuffer::from_attribute(
                Some(&format!("Attribute({})", self.name)),
                device,
                queue,
                data,
            ),
        )
    }

    /// 하위 메쉬 집합을 추가합니다.
    ///
    /// # Debug
    /// 주어진 인덱스 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn add_submesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: Indices) {
        self.submeshes.push(IndexBuffer::new(
            Some(&format!("Index({})", self.name)),
            device,
            queue,
            data,
        ));
    }

    /// 메쉬의 이름을 반환합니다.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 정점의 개수를 반환합니다.
    pub fn num_vertices(&self) -> u32 {
        self.num_vertices
    }

    /// 주어진 범위로 슬라이스된 정점 버퍼를 반환합니다.
    pub fn vertex<S>(&self, bounds: S) -> wgpu::BufferSlice
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.vertex.slice(bounds)
    }

    /// 주어진 범위로 슬라이스된 정점 속성 버퍼를 반환합니다.
    pub fn attribute<S>(&self, kind: &AttributeKind, bounds: S) -> Option<wgpu::BufferSlice>
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.attributes.get(kind).map(|buffer| buffer.slice(bounds))
    }

    /// 하위 메쉬 집합을 반환합니다.
    pub fn submeshes(&self) -> &[IndexBuffer] {
        &self.submeshes
    }
}

impl Hash for Mesh {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
