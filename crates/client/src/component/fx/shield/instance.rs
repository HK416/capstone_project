use std::{
    num::NonZeroU32,
    sync::{
        atomic::{AtomicU32, Ordering as MemOrdering},
        Arc,
    },
};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// 방어막 이펙트의 인스턴스 데이터 레이아웃입니다.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FxShieldInstanceDataLayout {
    pub x_axis: [f32; 4],
    pub y_axis: [f32; 4],
    pub z_axis: [f32; 4],
    pub w_axis: [f32; 4],
}

impl Default for FxShieldInstanceDataLayout {
    fn default() -> Self {
        Self {
            x_axis: [1.0, 0.0, 0.0, 0.0],
            y_axis: [0.0, 1.0, 0.0, 0.0],
            z_axis: [0.0, 0.0, 1.0, 0.0],
            w_axis: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// 방어막 파티클 이펙트의 인스턴스 버퍼입니다.
#[derive(Debug)]
pub struct FxShieldInstance {
    buffer: Arc<wgpu::Buffer>,
    capacity: u32,
    num_instance: AtomicU32,
}

impl FxShieldInstance {
    /// 인스턴스 버퍼 요소의 크기
    const ELEMENT_SIZE: wgpu::BufferAddress =
        core::mem::size_of::<FxShieldInstanceDataLayout>() as wgpu::BufferAddress;

    /// 인스턴스 버퍼의 [`wgpu::BufferUsages`]
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::VERTEX
        .union(wgpu::BufferUsages::COPY_DST)
        .union(wgpu::BufferUsages::MAP_WRITE);

    /// 새로운 인스턴스 버퍼를 생성합니다.
    pub fn new(device: &wgpu::Device, capacity: NonZeroU32) -> Self {
        let capacity = capacity.get();
        Self {
            buffer: Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance(Fx(Shield))"),
                mapped_at_creation: false,
                size: Self::ELEMENT_SIZE * capacity as u64,
                usage: Self::USAGES,
            })),
            capacity,
            num_instance: AtomicU32::new(0),
        }
    }

    /// 인스턴스 버퍼의 데이터를 지웁니다.  
    /// 실제 데이터는 남아있으며, 인스턴스의 개수만 초기화합니다.
    pub fn fast_clear(self) -> Self {
        Self {
            buffer: self.buffer,
            capacity: self.capacity,
            num_instance: AtomicU32::new(0),
        }
    }

    /// 인스턴스 버퍼 뷰를 가져옵니다.
    ///
    /// 인스턴스 용량이 부족한 경우 [`panic!`]을 호출합니다.
    ///
    pub fn get(&self) -> FxShieldInstanceView {
        let index = self.num_instance.fetch_add(1, MemOrdering::AcqRel);
        assert!(index < self.capacity, "out of bounds!");
        FxShieldInstanceView {
            buffer: self.buffer.clone(),
            offset: Self::ELEMENT_SIZE * index as u64,
        }
    }

    /// 인스턴스의 개수를 반환합니다.
    pub fn num_instance(&self) -> u32 {
        self.num_instance.load(MemOrdering::Acquire)
    }

    /// 인스턴스 범위에 해당하는 슬라이스된 버퍼를 반환합니다.
    pub fn slice(&self) -> wgpu::BufferSlice {
        let n = self.num_instance();
        assert!(n < self.capacity, "out of bounds!");
        let bytes = Self::ELEMENT_SIZE * n as u64;
        self.buffer.slice(..bytes)
    }
}

static_assertions::const_assert_ne!(FxShieldInstance::ELEMENT_SIZE, 0);
static_assertions::const_assert_eq!(
    FxShieldInstance::ELEMENT_SIZE as usize,
    core::mem::size_of::<FxShieldInstanceDataLayout>()
);

/// 인스턴스 버퍼의 뷰 입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxShieldInstanceView {
    buffer: Arc<wgpu::Buffer>,
    offset: u64,
}

impl FxShieldInstanceView {
    /// 인스턴스 버퍼에 데이터를 씁니다.
    pub fn write(
        self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: &FxShieldInstanceDataLayout,
    ) {
        // 스테이징 버퍼를 생성합니다.
        let contents = bytemuck::bytes_of(data);
        let copy_size = contents.len() as wgpu::BufferAddress;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Staging(Fx(Shield({})))", self.offset)),
            contents,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        encoder.copy_buffer_to_buffer(&buffer, 0, &self.buffer, self.offset, copy_size);
        staging_buffers.push(buffer);
    }
}
