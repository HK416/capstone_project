use std::hash;
use std::sync::Arc;
use std::cmp::Ordering;

use crate::components::VertexAttributeValues;



/// Mesh의 정점 버퍼 입니다.
#[derive(Debug)]
pub struct VertexAttribute {
    pub count: u32, 
    pub stride: u32, 
    pub buffer: wgpu::Buffer, 
    pub format: wgpu::VertexFormat, 
}

impl VertexAttribute {
    /// 새로운 정점 버퍼를 생성합니다.
    pub fn new(
        name: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: VertexAttributeValues
    ) -> Arc<Self> {
        // 라벨을 생성합니다.
        let label = format!("Attribute({})", name.unwrap_or("Unknown"));

        // GPU 메모리 버퍼를 생성합니다.
        let buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some(&label), 
                mapped_at_creation: false, 
                size: values.size() as wgpu::BufferAddress, 
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, 
            }
        );

        // 정점 데이터를 버퍼에 복사합니다.
        queue.write_buffer(&buffer, 0, values.as_bytes());

        Self {
            buffer, 
            count: values.count() as u32, 
            stride: values.stride() as u32, 
            format: values.format()
        }.into()
    }
}

impl Ord for VertexAttribute {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.buffer.global_id().cmp(&other.buffer.global_id())
    }
}

impl PartialOrd<Self> for VertexAttribute {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.buffer.global_id().partial_cmp(&other.buffer.global_id())
    }
}

impl Eq for VertexAttribute { }

impl PartialEq<Self> for VertexAttribute {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.buffer.global_id().eq(&other.buffer.global_id())
    }
}

impl hash::Hash for VertexAttribute {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.buffer.global_id().hash(state)
    }
}
