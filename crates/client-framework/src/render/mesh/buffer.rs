use std::ops;

use crate::render::mesh::Indices;
use crate::render::mesh::Vertices;
use crate::render::mesh::VertexAttributeValues;



/// 3차원 메쉬에서 사용되는 안댁스 버퍼입니다.
#[derive(Debug)]
pub struct IndexBuffer{
    count: u32, 
    buffer: wgpu::Buffer, 
}

impl IndexBuffer {
    /// 새로운 인덱스 버퍼를 생성합니다.
    #[must_use]
    pub fn new(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: Indices
    ) -> Self {
        // 인덱스 버퍼를 생성합니다.
        let buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label, 
                mapped_at_creation: false, 
                size: values.size() as wgpu::BufferAddress, 
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST, 
            }
        );

        // 인덱스 데이터를 정점 버퍼에 작성합니다.
        log::debug!("Write index buffer ({:?})", label);
        queue.write_buffer(&buffer, 0, values.as_bytes());

        Self { count: values.count() as u32, buffer }.into()
    }

    /// 인덱스의 갯수를 반환합니다.
    #[inline]
    #[must_use]
    pub fn count(&self) -> u32 {
        self.count
    }
}

impl ops::Deref for IndexBuffer {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl ops::DerefMut for IndexBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}



/// 3차원 메쉬에서 사용되는 정점 버퍼입니다.
#[derive(Debug)]
pub struct VertexBuffer(wgpu::Buffer);

impl VertexBuffer {
    pub fn from_vertices(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: Vertices
    ) -> Self {
        // 정점 버퍼를 생성합니다.
        let buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label, 
                mapped_at_creation: false, 
                size: values.size() as wgpu::BufferAddress, 
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, 
            }
        );

        // 정점 데이터를 정점 버퍼에 작성합니다.
        log::debug!("Write vertex buffer ({:?})", label);
        queue.write_buffer(&buffer, 0, values.as_bytes());

        Self(buffer).into()
    }

    /// 새로운 정점 속성 버퍼를 생성합니다.
    #[must_use]
    pub fn from_attribute(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: VertexAttributeValues
    ) -> Self {
        // 정점 버퍼를 생성합니다.
        let buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label, 
                mapped_at_creation: false, 
                size: values.size() as wgpu::BufferAddress, 
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, 
            }
        );

        // 정점 데이터를 정점 버퍼에 작성합니다.
        log::debug!("Write attribute vertex buffer ({:?})", label);
        queue.write_buffer(&buffer, 0, values.as_bytes());

        Self(buffer).into()
    }
}

impl ops::Deref for VertexBuffer {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for VertexBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
