use std::ops;
use std::hash;
use std::sync::OnceLock;
use std::cmp::Ordering;
use wgpu::util::DeviceExt;
use gmm::{Float3, Float4};



/// spot 조명 입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpotLight {
    /// 조명의 색상입니다.
    pub color: Float4, 

    /// 월드 좌표계상 조명의 위치 입니다.
    pub position: Float3, 
    
    /// 월드 좌표계상 조명이 퍼지는 각도입니다. (radians)
    pub angle: f32, 

    /// 월드 좌표계상 조명의 방향입니다.
    pub direction: Float3, 

    /// 월드 좌표계상 조명의 영향을 받는 거리 입니다.
    pub range: f32, 
}

impl Default for SpotLight {
    #[inline]
    fn default() -> Self {
        Self { 
            color: Float4::ONE, 
            position: Float3::ZERO, 
            angle: 0.0, 
            direction: Float3::ZERO, 
            range: 0.0 
        }
    }
}



/// 쉐이더에서 사용되는 spot 조명 변수에 대한 레이아웃 입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpotLightDataLayout {
    /// 사용되는 조명의 갯수 입니다.
    pub num_lights: u32, 
    pub _padding0: [u8; 12], 

    /// 사용되는 조명의 데이터 입니다.
    pub lights: [SpotLight; 16], 
}

impl Default for SpotLightDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            num_lights: 0, 
            lights: [SpotLight::default(); 16], 
            _padding0: [0; 12], 
        }
    }
}



/// 쉐이더에서 사용하는 spot 조명 유니폼 버퍼 입니다.
/// 
/// spot 조명 유니폼 버퍼는 애플리케이션에서 오직 한개만 존재합니다.
/// 
#[derive(Debug)]
pub struct SpotLightUniform(wgpu::Buffer);

impl SpotLightUniform {
    /// spot 조명의 유니폼 버퍼를 가져옵니다.
    #[must_use]
    pub fn get(device: &wgpu::Device) -> &Self {
        static INSTANCE: OnceLock<SpotLightUniform> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            SpotLightUniform(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Uniform(SpotLights)"), 
                    contents: bytemuck::bytes_of(&SpotLightDataLayout::default()), 
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, 
                }, 
            ))
        })
    }

    /// 유니폼 버퍼의 내용을 초기화 합니다.
    #[inline]
    pub fn reset(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self, 0, bytemuck::bytes_of(&SpotLightDataLayout::default()))
    }
}

impl ops::Deref for SpotLightUniform {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for SpotLightUniform {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.global_id().cmp(&other.global_id())
    }
}

impl PartialOrd<Self> for SpotLightUniform {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.global_id().partial_cmp(&other.global_id())
    }
}

impl Eq for SpotLightUniform { }

impl PartialEq<Self> for SpotLightUniform {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.global_id().eq(&other.global_id())
    }
}

impl hash::Hash for SpotLightUniform {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.global_id().hash(state)
    }
}
