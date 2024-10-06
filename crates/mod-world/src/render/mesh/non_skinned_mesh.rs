use std::sync::{Arc, OnceLock};

use super::{MeshUniform, ModelMesh};



/// 스키닝되지 않은 메쉬 데이터입니다.
#[derive(Debug)]
pub struct NonSkinnedMesh {
    /// 공유 메쉬 데이터입니다.
    pub(super) model_mesh: Arc<ModelMesh>, 

    /// 메쉬의 유니폼 버퍼입니다.
    pub(super) mesh_uniform: MeshUniform, 

    /// 메쉬의 바인드 그룹입니다.
    pub(super) bind_group: wgpu::BindGroup, 
}

impl NonSkinnedMesh {
    /// 스키닝되지 않은 메쉬의 [wgpu::BindGroupLayout]을 가져옵니다.
    #[must_use]
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(NonSkinnedMesh)"), 
                    entries: &[
                        // 0번 바인딩: 스키닝되지 않은 유니폼 버퍼
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
                    ]
                }
            )
        })
    }
}

impl NonSkinnedMesh {
    /// 모델 메쉬 데이터를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn model_mesh(&self) -> &Arc<ModelMesh> {
        &self.model_mesh
    }

    /// 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn mesh_uniform(&self) -> &MeshUniform {
        &self.mesh_uniform
    }

    /// 바인드 그룹을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
