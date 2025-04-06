//! 메쉬와 관련된 코드를 관리합니다.
//!
#![allow(dead_code)]
mod attribute;
mod index;
mod resource;
mod system;
mod uniform;

use std::{
    hash::{Hash, Hasher},
    ops::RangeBounds,
};

use ahash::{HashMap, RandomState};

pub use self::{attribute::*, index::*, resource::*, system::*, uniform::*};

/// 모델 메쉬 데이터입니다.
#[derive(Debug, PartialEq, Eq)]
pub struct Mesh {
    uri: String,
    num_vertices: u32,
    vertex: VertexBuffer,
    attributes: HashMap<AttributeKind, VertexBuffer>,
    submeshes: Vec<IndexBuffer>,
}

impl Mesh {
    /// 새로운 메쉬를 생성합니다.
    ///
    /// # Panics
    /// 주어진 정점 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new<Uri>(
        uri: Uri,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Vertices,
    ) -> Self
    where
        Uri: AsRef<str>,
    {
        const DEFAUL_CAPACITY: usize = 8;
        log::debug!("create mesh (URI:{})", uri.as_ref());
        Self {
            uri: uri.as_ref().into(),
            num_vertices: data.count() as u32,
            vertex: VertexBuffer::from_vertices(
                Some(uri.as_ref()),
                device,
                encoder,
                staging_buffers,
                data,
            ),
            attributes: HashMap::with_capacity_and_hasher(DEFAUL_CAPACITY, RandomState::new()),
            submeshes: Vec::with_capacity(DEFAUL_CAPACITY),
        }
    }

    /// 정점 속성을 추가합니다.  
    /// 이미 해당 정점 속성이 존재하는 경우 새로운 정점 속성으로 교체됩니다.
    ///
    /// # Panics
    /// 주어진 정점 속성 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn with_attribute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Attributes,
    ) -> &mut Self {
        self.attributes.insert(
            data.kind(),
            VertexBuffer::from_attribute(Some(&self.uri), device, encoder, staging_buffers, data),
        );
        self
    }

    /// 하위 메쉬 집합을 추가합니다.
    ///
    /// # Panics
    /// 주어진 인덱스 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn with_submesh(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Indices,
    ) -> &mut Self {
        self.submeshes.push(IndexBuffer::new(
            Some(&self.uri),
            device,
            encoder,
            staging_buffers,
            data,
        ));
        self
    }

    /// 메쉬의 Uri를 반환합니다.
    pub fn uri(&self) -> &str {
        &self.uri
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
        self.uri.hash(state);
    }
}
