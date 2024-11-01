use std::{collections::HashMap, mem, num::NonZeroU64, ops::{self, RangeBounds}, sync::{Arc, OnceLock}};

use bytemuck::{Pod, Zeroable};



/// 정점 속성 식별자 목록
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Attribute {
    Color, 
    Normal, 
    Tangent, 
    Texcoord0, 
    Texcoord1, 
    BoneIndex, 
    BoneWeight 
}





/// 인덱스 데이터
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Indices {
    U16(Vec<u16>), 
    U32(Vec<u32>) 
}

impl Indices {
    /// 인덱스 데이터가 비어있는 경우 `true`를 반환합니다.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 인덱스 데이터의 바이트 단위 크기를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 인덱스 데이터 요소의 개수를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        match self {
            Indices::U16(indices) => indices.len(),
            Indices::U32(indices) => indices.len(),
        }
    }

    /// 인덱스 데이터의 형식을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn format(&self) -> wgpu::IndexFormat {
        match self {
            Indices::U16(_) => wgpu::IndexFormat::Uint16,
            Indices::U32(_) => wgpu::IndexFormat::Uint32,
        }
    }

    /// 인덱스 데이터의 바이트 단위 크기를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn stride(&self) -> usize {
        match self {
            Indices::U16(_) => mem::size_of::<u16>(),
            Indices::U32(_) => mem::size_of::<u32>(),
        }
    }

    /// 인덱스 데이터의 바이트 배열을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Indices::U16(indices) => bytemuck::cast_slice(indices),
            Indices::U32(indices) => bytemuck::cast_slice(indices),
        }
    }
}





/// 메쉬에서 사용하는 인덱스 버퍼
#[derive(Debug)]
pub struct IndexBuffer {
    /// 인덱스 요소의 개수입니다.
    count: u32, 

    /// 인덱스 버퍼 형식입니다.
    format: wgpu::IndexFormat, 

    /// 인덱스 버퍼입니다.
    inner: wgpu::Buffer
}

impl IndexBuffer {
    /// 새로운 인덱스 버퍼를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 인덱스 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn new(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        data: Indices
    ) -> Self {
        assert!(!data.is_empty(), "The given index buffer data is empty!");
        unsafe { Self::new_unchecked(label, device, queue, data) }
    }

    /// 새로운 인덱스 버퍼를 생성합니다.
    /// 
    /// 이 함수는 인덱스 데이터가 비어있는지 확인하지 않습니다.
    /// 
    #[must_use]
    pub unsafe fn new_unchecked(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        data: Indices
    ) -> Self {
        // 인덱스 버퍼를 생성합니다.
        let buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label, 
                mapped_at_creation: false, 
                size: data.size() as wgpu::BufferAddress, 
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST 
            }
        );

        // 인덱스 버퍼 데이터를 작성합니다.
        queue.write_buffer(&buffer, 0, data.as_bytes());

        Self { 
            count: data.count() as u32, 
            format: data.format(), 
            inner: buffer 
        }
    }

    /// 인덱스 요소의 개수를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn count(&self) -> u32 {
        self.count
    }

    /// 인덱스 버퍼 형식을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn format(&self) -> wgpu::IndexFormat {
        self.format
    }

    /// 범위에 해당하는 인덱스 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice<'_> 
    where S: RangeBounds<wgpu::BufferAddress> {
        self.inner.slice(bounds)
    }
}





/// 정점 데이터
#[derive(Debug, Clone, PartialEq)]
pub struct Vertices(pub Vec<gmm::Float3>);

impl Vertices {
    /// 정점 데이터가 비어있는 경우 `true`를 반환합니다.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 정점 데이터의 바이트 단위 크기를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 정점 데이터 요소의 개수를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        self.0.len()
    }

    /// 정점 데이터 요소의 바이트 단위 크기를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn stride(&self) -> usize {
        mem::size_of::<gmm::Float3>()
    }

    /// 정점 데이터의 바이트 배열을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.0)
    }
}





/// 정점 속성 데이터
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValues {
    Color(Vec<gmm::Float4>), 
    Normal(Vec<gmm::Float3>), 
    Tangent(Vec<gmm::Float3>), 
    Texcoord0(Vec<gmm::Float2>), 
    Texcoord1(Vec<gmm::Float2>), 
    BoneIndex(Vec<gmm::UInteger4>), 
    BoneWeight(Vec<gmm::Float4>), 
}

impl AttributeValues {
    /// 정점 속성 데이터가 비어있는 경우 `true`를 반환합니다.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 정점 속성 데이터의 바이트 단위 크기를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 정점 속성 식별자를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn attribute(&self) -> Attribute {
        match self {
            AttributeValues::Color(_) => Attribute::Color,
            AttributeValues::Normal(_) => Attribute::Normal,
            AttributeValues::Tangent(_) => Attribute::Tangent,
            AttributeValues::Texcoord0(_) => Attribute::Texcoord0,
            AttributeValues::Texcoord1(_) => Attribute::Texcoord1,
            AttributeValues::BoneIndex(_) => Attribute::BoneIndex,
            AttributeValues::BoneWeight(_) => Attribute::BoneWeight,
        }
    }

    /// 정점 속성 데이터 요소의 개수를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        match self {
            AttributeValues::Color(values) => values.len(),
            AttributeValues::Normal(values) => values.len(),
            AttributeValues::Tangent(values) => values.len(),
            AttributeValues::Texcoord0(values) => values.len(),
            AttributeValues::Texcoord1(values) => values.len(),
            AttributeValues::BoneIndex(values) => values.len(),
            AttributeValues::BoneWeight(values) => values.len(),
        }
    }

    /// 정점 속성 데이터 요소의 바이트 단위 크기를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn stride(&self) -> usize {
        match self {
            AttributeValues::Color(_) => mem::size_of::<gmm::Float4>(),
            AttributeValues::Normal(_) => mem::size_of::<gmm::Float3>(),
            AttributeValues::Tangent(_) => mem::size_of::<gmm::Float3>(),
            AttributeValues::Texcoord0(_) => mem::size_of::<gmm::Float2>(),
            AttributeValues::Texcoord1(_) => mem::size_of::<gmm::Float2>(),
            AttributeValues::BoneIndex(_) => mem::size_of::<gmm::UInteger4>(),
            AttributeValues::BoneWeight(_) => mem::size_of::<gmm::Float4>(),
        }
    }

    /// 정점 속성 데이터의 바이트 배열을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            AttributeValues::Color(values) => bytemuck::cast_slice(values),
            AttributeValues::Normal(values) => bytemuck::cast_slice(values),
            AttributeValues::Tangent(values) => bytemuck::cast_slice(values),
            AttributeValues::Texcoord0(values) => bytemuck::cast_slice(values),
            AttributeValues::Texcoord1(values) => bytemuck::cast_slice(values),
            AttributeValues::BoneIndex(values) => bytemuck::cast_slice(values),
            AttributeValues::BoneWeight(values) => bytemuck::cast_slice(values),
        }
    }
}





/// 메쉬에서 사용하는 정점 버퍼
#[derive(Debug)]
pub struct VertexBuffer {
    inner: wgpu::Buffer
}

impl VertexBuffer {
    /// 정점 데이터로부터 정점 버퍼를 생성합니다.
    /// 
    /// # Panics
    /// 정점 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn from_vertices(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        data: Vertices
    ) -> Self {
        assert!(!data.is_empty(), "The given vertex data is empty!");
        unsafe { Self::from_vertices_unchecked(label, device, queue, data) }
    }

    /// 정점 데이터로부터 정점 버퍼를 생성합니다.
    /// 
    /// 이 함수는 정점 데이터가 비어있는지 확인하지 않습니다.
    /// 
    #[must_use]
    pub unsafe fn from_vertices_unchecked(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        data: Vertices
    ) -> Self {
        // 정점 버퍼를 생성합니다.
        let buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label, 
                mapped_at_creation: false, 
                size: data.size() as wgpu::BufferAddress, 
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST 
            }
        );

        // 정점 버퍼에 데이터를 작성합니다.
        queue.write_buffer(&buffer, 0, data.as_bytes());

        Self { inner: buffer }
    }

    /// 정점 속성 데이터로부터 정점 버퍼를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 정점 속성 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn from_attribute(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        data: AttributeValues
    ) -> Self {
        assert!(!data.is_empty(), "The given attribute data is empty!");
        unsafe { Self::from_attribute_unchecked(label, device, queue, data) }
    }

    /// 정점 속성 데이터로부터 정점 버퍼를 생성합니다.
    /// 
    /// 이 함수는 정점 속성 데이터가 비어있는지 확인하지 않습니다.
    /// 
    #[must_use]
    pub unsafe fn from_attribute_unchecked(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        data: AttributeValues
    ) -> Self {
        // 정점 버퍼를 생성합니다.
        let buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label, 
                mapped_at_creation: false, 
                size: data.size() as wgpu::BufferAddress, 
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST 
            }
        );

        // 정점 버퍼에 데이터를 작성합니다.
        queue.write_buffer(&buffer, 0, data.as_bytes());

        Self { inner: buffer }
    }

    /// 범위에 해당하는 정점 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice<'_> 
    where S: RangeBounds<wgpu::BufferAddress> {
        self.inner.slice(bounds)
    }
}





/// 3차원 메쉬
#[derive(Debug)]
pub struct Mesh {
    /// 메쉬의 이름입니다.
    name: String, 

    /// 메쉬의 정점 개수입니다.
    num_vertices: u32, 

    /// 메쉬의 정점 버퍼입니다.
    vertex: VertexBuffer, 

    /// 메쉬의 정점 속성 버퍼입니다.
    attributes: HashMap<Attribute, VertexBuffer>, 

    /// 메쉬의 하위 메쉬 집합입니다.
    submeshes: Vec<IndexBuffer>, 
}

impl Mesh {
    /// 새로운 메쉬를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 정점 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    /// 
    #[must_use]
    pub fn new(name: &str, device: &wgpu::Device, queue: &wgpu::Queue, data: Vertices) -> Self {
        Self { 
            name: name.to_string(), 
            num_vertices: data.count() as u32, 
            vertex: VertexBuffer::from_vertices(
                Some(&format!("Vertex({})", &name)), 
                device, 
                queue, 
                data
            ), 
            attributes: HashMap::with_capacity(8), 
            submeshes: Vec::with_capacity(8) 
        }
    }

    /// 메쉬에 정점 속성 버퍼를 추가합니다.
    /// 
    /// 이미 해당 정점 속성 버퍼를 갖고 있는 경우 정점 속성 버퍼가 교체됩니다.
    /// 
    /// # Panics
    /// 주어진 정점 속성 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    /// 
    pub fn insert_attribute(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: AttributeValues) -> Option<VertexBuffer> {
        self.attributes.insert(
            data.attribute(), 
            VertexBuffer::from_attribute(
                Some(&format!("Attribute({})", &self.name)), 
                device, 
                queue, 
                data
            )
        )
    }

    /// 메쉬에 하위 메쉬를 추가합니다.
    /// 
    /// # Panics
    /// 주어진 하위 메쉬 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    /// 
    pub fn insert_submesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: Indices) {
        self.submeshes.push(
            IndexBuffer::new(
                Some(&format!("Index({})", &self.name)), 
                device, 
                queue, 
                data
            )
        )
    }


    /// 메쉬의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 메쉬의 정점 개수를 가져옵니다.
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
    pub fn attribute(&self, attr: &Attribute) -> Option<&VertexBuffer> {
        self.attributes.get(attr)
    }

    /// 메쉬의 하위 메쉬 집합을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn submeshes(&self) -> &[IndexBuffer] {
        &self.submeshes
    }
}





/// 정적인 메쉬 데이터 레이아웃
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct StaticMeshDataLayout {
    pub trans: [f32; 16] 
}

impl Default for StaticMeshDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            trans: gmm::Float4x4::IDENTITY.into() 
        }
    }
}





/// 정적인 메쉬 데이터 유니폼 버퍼
#[derive(Debug)]
pub struct StaticMeshUniform {
    inner: Arc<wgpu::Buffer>
}

impl StaticMeshUniform {
    /// 유니폼 버퍼의 크기
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<StaticMeshDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl StaticMeshUniform {
    /// 초기화 되지 않은 정적인 메쉬 데이터 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self { 
            inner: device.create_buffer(
                &wgpu::BufferDescriptor {
                    label, 
                    mapped_at_creation: false, 
                    size: Self::SIZE, 
                    usage: Self::USAGES 
                }
            ).into() 
        }
    }

    /// 정적인 메쉬 데이터 유니폼 버퍼를 작성합니다.
    pub fn write(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: StaticMeshDataLayout) {
        let capturable = self.inner.clone();
        self.inner.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut StaticMeshDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (UNIFORM:{})", e);
                }
            }
        });

        // 제출된 작업이 끝날 때 까지 대기합니다.
        let index = queue.submit([]);
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(index));
    }

    /// 정적인 메쉬 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.inner
    }
}

static_assertions::const_assert_ne!(StaticMeshUniform::SIZE, 0);
static_assertions::const_assert_eq!(StaticMeshUniform::SIZE as usize, mem::size_of::<StaticMeshDataLayout>());





/// 정적인 메쉬 데이터 쉐이더 리소스
#[derive(Debug)]
pub struct StaticMeshResource {
    /// 정적인 메쉬 데이터 유니폼 버퍼입니다.
    mesh_uniform: StaticMeshUniform, 

    /// 정적인 메쉬의 [wgpu::BindGroup]입니다.
    bind_group: wgpu::BindGroup 
}

impl StaticMeshResource {
    /// 정적인 메쉬 데이터 쉐이더 리소스의 [wgpu::BindGroupLayout]을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(StaticMeshResource)"), 
                    entries: &[
                        // 0번 바인딩: 정적인 메쉬 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: unsafe {
                                    Some(NonZeroU64::new_unchecked(StaticMeshUniform::SIZE))
                                } 
                            }, 
                            count: None 
                        }
                    ]
                }
            )
        })
    }
}

impl StaticMeshResource {
    /// 새로운 정적인 메쉬 데이터 쉐이더 리소스를 생성합니다.
    #[must_use]
    pub fn new(name: Option<&str>, device: &wgpu::Device) -> Self {
        let name = name.unwrap_or("Unknown");
        let mesh_uniform = StaticMeshUniform::new(
            Some(&format!("StaticMeshUniform({})", &name)), 
            device
        );
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup(StaticMeshResource({}))", &name)), 
                layout: &Self::bind_group_layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: mesh_uniform.buffer().as_entire_binding() 
                    }
                ]
            }
        );

        Self { 
            mesh_uniform, 
            bind_group 
        }
    }

    /// 정적인 메쉬 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn mesh_uniform(&self) -> &StaticMeshUniform {
        &self.mesh_uniform
    }

    /// 정적인 메쉬 데이터의 [wgpu::BindGroup]을 가져옵니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}





/// 동적인 메쉬 데이터 레이아웃
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DynamicMeshDataLayout {
    /// 정점에 연결된 뼈의 개수 (0..=4)
    pub quality: u32, 

    /// 메쉬에 포함된 뼈의 개수
    pub num_bones: u32, 
    pub _padding0: [u8; 8]
}

impl Default for DynamicMeshDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            quality: 0, 
            num_bones: 0, 
            _padding0: [0; 8] 
        }
    }
}





/// 동적인 메쉬 데이터 유니폼 버퍼
#[derive(Debug, Clone)]
pub struct DynamicMeshUniform {
    inner: Arc<wgpu::Buffer>
}

impl DynamicMeshUniform {
    /// 유니폼 버퍼의 크기
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<DynamicMeshDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl DynamicMeshUniform {
    /// 초기화 되지 않은 동적인 메쉬 데이터 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self { 
            inner: device.create_buffer(
                &wgpu::BufferDescriptor {
                    label, 
                    mapped_at_creation: false, 
                    size: Self::SIZE, 
                    usage: Self::USAGES 
                }
            ).into() 
        }
    }

    /// 동적인 메쉬 데이터 유니폼 버퍼를 작성합니다.
    pub fn write(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: DynamicMeshDataLayout) {
        let capturable = self.inner.clone();
        self.inner.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut DynamicMeshDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (UNIFORM:{})", e);
                }
            }
        });

        // 제출된 작업이 끝날 때 까지 대기합니다.
        let index = queue.submit([]);
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(index));
    }

    /// 동적인 메쉬 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.inner
    }
}

static_assertions::const_assert_ne!(DynamicMeshUniform::SIZE, 0);
static_assertions::const_assert_eq!(DynamicMeshUniform::SIZE as usize, mem::size_of::<DynamicMeshDataLayout>());





/// 최대 뼈의 개수
pub const MAX_BONES: usize = 256;

/// 뼈 변환 행렬 데이터 레이아웃
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BoneMatrixDataLayout {
    pub arr: [[f32; 16]; MAX_BONES] 
}

impl BoneMatrixDataLayout {
    /// 새로운 뼈 변환 행렬 데이터 레이아웃을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = &[f32; 16]> {
        self.arr.iter()
    }

    #[inline]
    #[must_use]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut [f32; 16]> {
        self.arr.iter_mut()
    }
}

impl ops::Index<usize> for BoneMatrixDataLayout {
    type Output = [f32; 16];
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.arr.get(index).expect("out of bounds!")
    }
}

impl ops::IndexMut<usize> for BoneMatrixDataLayout {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.arr.get_mut(index).expect("out of bounds!")
    }
}

impl Default for BoneMatrixDataLayout {
    #[inline]
    fn default() -> Self {
        Self { arr: [gmm::Float4x4::IDENTITY.into(); MAX_BONES] }
    }
}





/// 뼈 변환 행렬 데이터 유니폼 버퍼
#[derive(Debug, Clone)]
pub struct BoneMatrixUniform {
    inner: Arc<wgpu::Buffer>
}

impl BoneMatrixUniform {
    /// 유니폼 버퍼의 크기
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<BoneMatrixDataLayout>() as wgpu::BufferAddress;

    /// 유니폼 버퍼의 [wgpu::BufferUsages]
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl BoneMatrixUniform {
    /// 초기화 되지 않은 뼈 변한 행렬 데이터 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &wgpu::Device) -> Self {
        Self { 
            inner: device.create_buffer(
                &wgpu::BufferDescriptor {
                    label, 
                    mapped_at_creation: false, 
                    size: Self::SIZE, 
                    usage: Self::USAGES 
                }
            ).into() 
        }
    }

    /// 뼈 변한 행렬 데이터 유니폼 버퍼를 작성합니다.
    pub fn write(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: BoneMatrixDataLayout) {
        let capturable = self.inner.clone();
        self.inner.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut BoneMatrixDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (UNIFORM:{})", e);
                }
            }
        });

        // 제출된 작업이 끝날 때 까지 대기합니다.
        let index = queue.submit([]);
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(index));
    }

    /// 뼈 변환 행렬 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.inner
    }
}

static_assertions::const_assert_ne!(BoneMatrixUniform::SIZE, 0);
static_assertions::const_assert_eq!(BoneMatrixUniform::SIZE as usize, mem::size_of::<BoneMatrixDataLayout>());





/// 동적인 메쉬 데이터 쉐이더 리소스
#[derive(Debug)]
pub struct DynamicMeshResource {
    /// 동적인 메쉬 데이터 유니폼 버퍼입니다.
    mesh_uniform: DynamicMeshUniform, 

    /// 뼈 바인드 포즈 데이터 유니폼 버퍼입니다.
    bindpose_uniform: BoneMatrixUniform, 

    /// 뼈 변환 행렬 데이터 유니폼 버퍼입니다.
    bone_transform_uniform: BoneMatrixUniform, 

    /// 동적인 메쉬의 [wgpu::BindGroup]입니다.
    bind_group: wgpu::BindGroup 
}

impl DynamicMeshResource {
    /// 동적인 메쉬 데이터 쉐이더 리소스의 [wgpu::BindGroupLayout]을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(DynamicMeshResource)"), 
                    entries: &[
                        // 0번 바인딩: 동적인 메쉬 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: unsafe {
                                    Some(NonZeroU64::new_unchecked(DynamicMeshUniform::SIZE))
                                } 
                            }, 
                            count: None
                        }, 
                        // 1번 바인딩: 뼈 바인드 포즈 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: unsafe {
                                    Some(NonZeroU64::new_unchecked(BoneMatrixUniform::SIZE))
                                } 
                            }, 
                            count: None
                        }, 
                        // 2번 데이터: 뼈 변환 행렬 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,  
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: unsafe {
                                    Some(NonZeroU64::new_unchecked(BoneMatrixUniform::SIZE))
                                } 
                            }, 
                            count: None
                        }
                    ]
                }
            )
        })
    }
}

impl DynamicMeshResource {
    /// 새로운 동적인 메쉬 데이터 쉐이더 리소스를 생성합니다.
    #[must_use]
    pub fn new(name: Option<&str>, device: &wgpu::Device) -> Self {
        let name = name.unwrap_or("Unknown");
        let mesh_uniform = DynamicMeshUniform::new(
            Some(&format!("DynamicMeshUniform({})", &name)), 
            device
        );
        let bindpose_uniform = BoneMatrixUniform::new(
            Some(&format!("BindposeUniform({})", &name)), 
            device
        );
        let bone_transform_uniform = BoneMatrixUniform::new(
            Some(&format!("BoneTransformUniform({})", &name)), 
            device
        );
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup(DynamicMeshResource({}))", &name)), 
                layout: &Self::bind_group_layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: mesh_uniform.buffer().as_entire_binding() 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: bindpose_uniform.buffer().as_entire_binding()
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: bone_transform_uniform.buffer().as_entire_binding() 
                    } 
                ]
            }
        );

        Self { 
            mesh_uniform, 
            bindpose_uniform, 
            bone_transform_uniform, 
            bind_group 
        }
    }

    /// 동적인 메쉬 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn mesh_uniform(&self) -> &DynamicMeshUniform {
        &self.mesh_uniform
    }

    /// 뼈 바인드 포즈 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bindpose_uniform(&self) -> &BoneMatrixUniform {
        &self.bindpose_uniform
    }

    /// 뼈 변환 행렬 데이터 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bone_transform_uniform(&self) -> &BoneMatrixUniform {
        &self.bone_transform_uniform
    }

    /// 동적인 메쉬 데이터의 [wgpu::BindGroup]을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
