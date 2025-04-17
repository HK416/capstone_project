#![allow(dead_code)]
//! 인덱스 버퍼와 관련된 코드를 관리합니다.
//!

use std::ops::RangeBounds;

use wgpu::util::DeviceExt;

/// 인덱스 버퍼 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Indices(pub Vec<u32>);

impl Indices {
    /// 인덱스 데이터가 비어있는 경우 `true`를 반환합니다.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 인덱스 데이터 요소의 개수를 반환합니다.
    pub fn count(&self) -> usize {
        self.0.len()
    }

    /// 인덱스 데이터의 바이트 단위 크기를 반환합니다.
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 인덱스 데이터 요소의 바이트 단위 크기르 반환합니다.
    pub fn stride(&self) -> usize {
        core::mem::size_of::<u32>()
    }

    /// 인덱스 데이터의 인덱스 포맷을 반환합니다.
    pub fn format(&self) -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }

    /// 인덱스 데이터의 바이트 배열을 반환합니다.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.0)
    }
}

/// 인덱스 버퍼입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexBuffer {
    count: u32,
    format: wgpu::IndexFormat,
    buffer: wgpu::Buffer,
}

impl IndexBuffer {
    /// 새로운 인덱스 버퍼를 생성합니다.
    ///
    /// # Panics
    /// 주어진 인덱스 버퍼 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Indices,
    ) -> Self {
        assert!(!data.is_empty(), "the given index buffer data is empty!");
        unsafe { Self::new_unchecked(label, device, encoder, staging_buffers, data) }
    }

    /// 새로운 인덱스 버퍼를 생성합니다.
    ///
    /// # Safety
    /// 주어진 인덱스 버퍼 데이터가 비어있는 경우 정의되지 않은 동작을 수행합니다.
    ///
    pub unsafe fn new_unchecked(
        label: Option<&str>,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: Indices,
    ) -> Self {
        log::debug!("create index buffer (Label:{})", label.unwrap_or("Unknwon"));

        // 스테이징(업로드) 버퍼를 생성합니다.
        let count = data.count() as u32;
        let format = data.format();
        let data_size = data.size() as wgpu::BufferAddress;
        let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Staging(Index({}))", label.unwrap_or("Unknown"))),
            contents: data.as_bytes(),
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // 인덱스 버퍼를 생성합니다.
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Index({})", label.unwrap_or("Unknown"))),
            mapped_at_creation: false,
            size: data_size,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        // 버퍼의 데이터를 복사합니다.
        encoder.copy_buffer_to_buffer(&staging, 0, &buffer, 0, data_size);
        staging_buffers.push(staging);

        Self {
            count,
            format,
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
