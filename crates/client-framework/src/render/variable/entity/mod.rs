use std::ops;
use std::hash;
use std::sync::Arc;
use std::cmp::Ordering;
use wgpu::util::DeviceExt;
use gmm::{Float3, Float4x4};
use bitflags::bitflags;



#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureFlag(u32);

bitflags! {
    impl TextureFlag : u32 {
        const NONE = 0x00;
        const AMBIENT = 0x01;
        const DIFFUSE = 0x02;
        const NORMAL = 0x04;
        const SPECULAR = 0x08;
        const EMISSIVE = 0x10;
    }
}



/// 쉐이더에서 사용되는 오브젝트 변수에 대한 레이아웃 입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EntityDataLayout {
    /// 월드 변환 행렬 입니다.
    pub trans: Float4x4, 

    /// 월드 좌표계상 오브젝트의 위치 입니다.
    pub position: Float3, 

    /// 어떤 텍스처가 사용되는지 나타내는 플래그 입니다.
    pub texture_flag: u32, 
}

impl Default for EntityDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            trans: Float4x4::IDENTITY, 
            position: Float3::ZERO, 
            texture_flag: 0, 
        }
    }
}



/// 쉐이더에서 사용하는 오브젝트 유니폼 버퍼 입니다.
#[derive(Debug)]
pub struct EntityUniform(wgpu::Buffer);

impl EntityUniform {
    /// 오브젝트 데이터로부터 오브젝트 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn from_data(
        name: Option<&str>, 
        device: &wgpu::Device, 
        data: EntityDataLayout
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

impl ops::Deref for EntityUniform {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for EntityUniform {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.global_id().cmp(&other.global_id())
    }
}

impl PartialOrd<Self> for EntityUniform {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.global_id().partial_cmp(&other.global_id())
    }
}

impl Eq for EntityUniform { }

impl PartialEq<Self> for EntityUniform {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.global_id().eq(&other.global_id())
    }
}

impl hash::Hash for EntityUniform {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.global_id().hash(state)
    }
}
