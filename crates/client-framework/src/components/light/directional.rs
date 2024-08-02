use std::ops;
use std::hash;
use std::sync::OnceLock;
use std::cmp::Ordering;
use wgpu::util::DeviceExt;
use gmm::{Float3, Float4};



/// 쉐이더에서 사용되는 방향 조명 변수에 대한 레이아웃 입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DirLightDataLayout {
    /// 조명의 색상입니다.
    pub color: Float4, 

    /// 월드 좌표계상 조명의 방향입니다.
    pub direction: Float3, 
    pub _padding0: [u8; 4], 
}

impl Default for DirLightDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            color: Float4::ONE, 
            direction: Float3::ZERO, 
            _padding0: [0; 4], 
        }
    }
}



/// 쉐이더에서 사용하는 방향 조명 유니폼 버퍼 입니다.
/// 
/// 방향 조명 유니폼 버퍼는 애플리케이션에서 오직 한개만 존재합니다.
/// 
#[derive(Debug)]
pub struct DirLightUniform(wgpu::Buffer);

impl DirLightUniform {
    /// 방향 조명의 유니폼 버퍼를 가져옵니다.
    #[must_use]
    pub fn get(device: &wgpu::Device) -> &Self {
        static INSTANCE: OnceLock<DirLightUniform> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            DirLightUniform(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Uniform(DirectionalLight)"), 
                    contents: bytemuck::bytes_of(&DirLightDataLayout::default()), 
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, 
                }
            ))
        })
    }

    /// 유니폼 버퍼의 내용을 초기화 합니다.
    #[inline]
    pub fn reset(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self, 0, bytemuck::bytes_of(&DirLightDataLayout::default()))
    }
}

impl ops::Deref for DirLightUniform {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for DirLightUniform {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.global_id().cmp(&other.global_id())
    }
}

impl PartialOrd<Self> for DirLightUniform {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.global_id().partial_cmp(&other.global_id())
    }
}

impl Eq for DirLightUniform { }

impl PartialEq<Self> for DirLightUniform {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.global_id().eq(&other.global_id())
    }
}

impl hash::Hash for DirLightUniform {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.global_id().hash(state)
    }
}
