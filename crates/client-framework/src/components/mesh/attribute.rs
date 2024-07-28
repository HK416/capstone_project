use std::cmp;
use std::ops;
use std::hash;
use std::sync::Arc;
use super::VertexAttributeValues;



#[derive(Debug)]
pub struct VertexAttribute {
    count: u32, 
    stride: u32, 
    buffer: wgpu::Buffer, 
}

impl VertexAttribute {
    /// 새로운 정점 속성을 생성합니다.
    pub fn new(
        label: Option<&str>, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        value: VertexAttributeValues
    ) -> Arc<Self> {
        // GPU 전용 정점 버퍼를 생성합니다.
        let buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some(&format!("Buffer({})", label.unwrap_or("Unknown"))), 
                mapped_at_creation: false, 
                size: value.size() as wgpu::BufferAddress, 
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, 
            }
        );

        // 정점 속성의 값을 정점 버퍼에 복사합니다.
        queue.write_buffer(&buffer, 0, value.as_bytes());

        Self {
            buffer, 
            count: value.count() as u32, 
            stride: value.stride() as u32, 
        }.into()
    }

    /// 정점 속성 요소의 갯수를 반환합니다.
    #[inline]
    pub fn count(&self) -> u32 {
        self.count
    }

    /// 정점 속성 요소의 크기를 반환합니다.
    #[inline]
    pub fn stride(&self) -> u32 {
        self.stride
    }

    /// 정점 속성을 렌더 상태머신에 바인드 합니다.
    #[inline]
    pub fn bind<'a>(&'a self, slot: u32, encoder: &mut dyn wgpu::util::RenderEncoder<'a>) {
        encoder.set_vertex_buffer(slot, self.buffer.slice(..));
    }
}

impl ops::Deref for VertexAttribute {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl Ord for VertexAttribute {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.buffer.global_id().cmp(&other.buffer.global_id())
    }
}

impl PartialOrd<Self> for VertexAttribute {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
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
