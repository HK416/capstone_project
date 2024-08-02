use std::ops;
use std::hash;
use std::sync::OnceLock;
use std::cmp::Ordering;
use wgpu::util::DeviceExt;
use gmm::{Float3, Float4};



/// 점 조명 입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLight {
    /// 조명의 색상입니다.
    pub color: Float4, 

    /// 월드 좌표계상 조명의 위치 입니다.
    pub position: Float3, 

    /// 월드 좌표계상 조명의 영향을 받는 거리 입니다.
    pub range: f32, 
}

impl Default for PointLight {
    #[inline]
    fn default() -> Self {
        Self {
            color: Float4::ONE, 
            position: Float3::ZERO, 
            range: 0.0, 
        }
    }
}



/// 쉐이더에서 사용되는 점 조명 변수에 대한 레이아웃 입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightDataLayout {
    /// 사용되는 조명의 갯수 입니다. (최대 16)
    pub num_lights: u32, 
    pub _padding0: [u8; 12],

    /// 사용되는 조명의 데이터 입니다.
    pub lights: [PointLight; 16], 
}

impl Default for PointLightDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            num_lights: 0, 
            lights: [PointLight::default(); 16], 
            _padding0: [0; 12], 
        }
    }
}



/// 쉐이더에서 사용하는 점 조명 유니폼 버퍼 입니다.
/// 
/// 점 조명 유니폼 버퍼는 애플리케이션에서 오직 한개만 존재합니다.
/// 
#[derive(Debug)]
pub struct PointLightUniform(wgpu::Buffer);

impl PointLightUniform {
    /// 점 조명의 유니폼 버퍼를 가져옵니다.
    #[must_use]
    pub fn get(device: &wgpu::Device) -> &Self {
        static INSTANCE: OnceLock<PointLightUniform> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            PointLightUniform(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Uniform(PointLights)"), 
                    contents: bytemuck::bytes_of(&PointLightDataLayout::default()), 
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, 
                }, 
            ))
        })
    }

    /// 유니폼 버퍼의 내용을 초기화 합니다.
    #[inline]
    pub fn reset(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self, 0, bytemuck::bytes_of(&PointLightDataLayout::default()))
    }
}

impl ops::Deref for PointLightUniform {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for PointLightUniform {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.global_id().cmp(&other.global_id())
    }
}

impl PartialOrd<Self> for PointLightUniform {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.global_id().partial_cmp(&other.global_id())
    }
}

impl Eq for PointLightUniform { }

impl PartialEq<Self> for PointLightUniform {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.global_id().eq(&other.global_id())
    }
}

impl hash::Hash for PointLightUniform {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.global_id().hash(state)
    }
}
