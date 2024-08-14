mod buffer;
pub use self::buffer::*;

mod layout;
pub use self::layout::*;

use std::sync::Arc;
use std::sync::OnceLock;



/// 3차원 메쉬의 스키닝 데이터입니다.
#[derive(Debug, Clone)]
pub struct Skinning {
    name: String, 
    buffer: Arc<BoneOffsetsBuffer>, 
    bind_group: Arc<wgpu::BindGroup>, 
}

impl Skinning {
    /// 뼈 오프셋의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(Skinning)"), 
                    entries: &[
                        // 0번 바인딩: 뼈 오프셋 행렬
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None, 
                        }, 
                    ], 
                }, 
            )
        })
    }
}

impl Skinning {
    /// 새로운 스키닝 데이터를 생성합니다.
    #[must_use]
    pub fn new(name: Option<&str>, device: &wgpu::Device) -> Self {
        // 디버깅 라벨을 생성합니다.
        let name = format!("Skinning({})", name.unwrap_or("Unknown"));

        // 유니폼 버퍼를 생성합니다.
        let buffer = BoneOffsetsBuffer::new(Some(&format!("Uniform({})", name)), device);

        // 바인드 그룹을 생성합니다.
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", name)), 
                layout: &Self::layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: buffer.as_entire_binding(), 
                    }, 
                ], 
            }, 
        ).into();

        Self { name, buffer, bind_group }.into()
    }
}

impl Skinning {
    /// 뼈 오프셋 이름을 반환합니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 뼈 오프셋 유니폼 버퍼를 반환합니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> Arc<BoneOffsetsBuffer> {
        self.buffer.clone()
    }

    /// 뼈 오프셋의 [wgpu::BindGroup]을 반환합니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> Arc<wgpu::BindGroup> {
        self.bind_group.clone()
    }
}
