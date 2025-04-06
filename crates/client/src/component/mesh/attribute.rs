//! 정점 버퍼와 정점 속성 버퍼에 관련된 코드를 관리합니다.
//!
#![allow(dead_code)]

use std::{fmt, ops::RangeBounds};

use wgpu::util::DeviceExt;

/// 정점 속성 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AttributeKind {
    Color = 0,
    Normal = 1,
    Tangent = 2,
    Texcoord0 = 3,
    Texcoord1 = 4,
    Texcoord2 = 5,
    Texcoord3 = 6,
    BoneIndex = 7,
    BoneWeight = 8,
}

impl fmt::Display for AttributeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AttributeKind::Color => "Color",
                AttributeKind::Normal => "Normal",
                AttributeKind::Tangent => "Tangent",
                AttributeKind::Texcoord0 => "Texcoord0",
                AttributeKind::Texcoord1 => "Texcoord1",
                AttributeKind::Texcoord2 => "Texcoord2",
                AttributeKind::Texcoord3 => "Texcoord3",
                AttributeKind::BoneIndex => "BoneIndex",
                AttributeKind::BoneWeight => "BoneWeight",
            }
        )
    }
}

/// 정점 속성 버퍼의 데이터입니다.
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
        }
    }
}

/// 정점 버퍼의 데이터입니다.
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

/// 정점 버퍼입니다.
#[derive(Debug, PartialEq, Eq)]
pub struct VertexBuffer(wgpu::Buffer);

impl VertexBuffer {
    /// 새로운 정점 버퍼를 생성합니다.
    ///
    /// # Panics
    /// 주어진 정점 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_vertices(
        label: Option<&str>,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Vertices,
    ) -> Self {
        assert!(!data.is_empty(), "the given vertex data is empty!");
        unsafe { Self::from_vertices_unchecked(label, device, encoder, staging_buffers, data) }
    }

    /// 새로운 정점 버퍼를 생성합니다.
    ///
    /// # Safety
    /// 주어진 정점 버퍼 데이터가 비어있는 경우 정의되지 않은 동작을 수행합니다.
    ///
    pub unsafe fn from_vertices_unchecked(
        label: Option<&str>,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Vertices,
    ) -> Self {
        log::debug!(
            "create vertex buffer (LABEL:{})",
            label.unwrap_or("Unknown")
        );

        // 스테이징(업로드) 버퍼를 생성합니다.
        let data_size = data.size() as wgpu::BufferAddress;
        let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Staging(Vertex({}))", label.unwrap_or("Unknown"))),
            contents: data.as_bytes(),
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // 정점 버퍼를 생성합니다.
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Vertex({})", label.unwrap_or("Unknown"))),
            mapped_at_creation: false,
            size: data_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // 버퍼의 데이터를 복사합니다.
        encoder.copy_buffer_to_buffer(&staging, 0, &buffer, 0, data_size);
        staging_buffers.push(staging);

        Self(buffer)
    }

    /// 새로운 정점 속성 버퍼를 생성합니다.
    ///
    /// # Panics
    /// 주어지 정점 속성 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn from_attribute(
        label: Option<&str>,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Attributes,
    ) -> Self {
        assert!(
            !data.is_empty(),
            "the given vertex attribute data is empty!"
        );
        unsafe { Self::from_attribute_unchecked(label, device, encoder, staging_buffers, data) }
    }

    /// 새로운 정점 속성 버퍼를 생성합니다.
    ///
    /// # Safety
    /// 주어지 정점 속성 버퍼 데이터가 비어있는 경우 정의되지 않은 동작을 수행합니다.
    ///
    pub unsafe fn from_attribute_unchecked(
        label: Option<&str>,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Attributes,
    ) -> Self {
        log::debug!(
            "create vertex attribute buffer (LABEL:{})",
            label.unwrap_or("Unknown")
        );

        // 스테이징(업로드) 버퍼를 생성합니다.
        let data_size = data.size() as wgpu::BufferAddress;
        let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!(
                "Staging({}({}))",
                data.kind(),
                label.unwrap_or("Unknown")
            )),
            contents: data.as_bytes(),
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // 정점 속성 버퍼를 생성합니다.
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{}({})", data.kind(), label.unwrap_or("Unknown"))),
            mapped_at_creation: false,
            size: data_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // 버퍼의 데이터를 복사합니다.
        encoder.copy_buffer_to_buffer(&staging, 0, &buffer, 0, data_size);
        staging_buffers.push(staging);

        Self(buffer)
    }

    /// 범위에 해당하는 슬라이스된 정점 버퍼를 가져옵니다.
    pub fn slice<S>(&self, bounds: S) -> wgpu::BufferSlice<'_>
    where
        S: RangeBounds<wgpu::BufferAddress>,
    {
        self.0.slice(bounds)
    }
}
