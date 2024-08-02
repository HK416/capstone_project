use std::ops;
use std::hash;
use std::sync::Arc;
use std::cmp::Ordering;
use gmm::{Float3, Float4x4};
use wgpu::util::DeviceExt;



/// 쉐이더에서 사용되는 카메라 변수에 대한 레이아웃 입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraDataLayout {
    /// 투영 변환 행렬과 카메라 변환 행렬을 곱한 변환 행렬 입니다.
    pub proj_view: Float4x4, 

    /// 월드 좌표계상 카메라의 위치 입니다.
    pub position: Float3, 
    pub _padding0: [u8; 4], 

    /// 월드 좌표계상 카메라가 바라보는 방향입니다.
    pub direction: Float3, 
    pub _padding1: [u8; 4], 
}

impl Default for CameraDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            proj_view: Float4x4::IDENTITY, 
            position: Float3::ZERO, 
            direction: Float3::NEG_Z, 
            _padding0: [0; 4], 
            _padding1: [0; 4] 
        }
    }
}



/// 쉐이더에서 사용하는 카메라 유니폼 버퍼 입니다.
#[derive(Debug)]
pub struct CameraUniform(wgpu::Buffer);

impl CameraUniform {
    /// 카메라 데이터로부터 카메라 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn from_data(
        name: Option<&str>, 
        device: &wgpu::Device, 
        data: CameraDataLayout
    ) -> Arc<Self> {
        // 라벨을 생성한다.
        let label = format!("Uniform({})", name.unwrap_or("Unknown"));

        // 버퍼를 생성합니다.
        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&label), 
                contents: bytemuck::bytes_of(&data), 
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
            },
        );

        Self(buffer).into()
    }
}

impl ops::Deref for CameraUniform {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for CameraUniform {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.global_id().cmp(&other.global_id())
    }
}

impl PartialOrd<Self> for CameraUniform {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.global_id().partial_cmp(&other.global_id())
    }
}

impl Eq for CameraUniform { }

impl PartialEq<Self> for CameraUniform {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.global_id().eq(&other.global_id())
    }
}

impl hash::Hash for CameraUniform {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.global_id().hash(state)
    }
}
