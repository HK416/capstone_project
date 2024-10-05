use std::sync::{Arc, OnceLock};

use crate::component::WorldID;

use super::{BoneUniform, ModelMesh, SkinnedMeshUniform};



/// 스키닝된 메쉬 데이터입니다.
#[derive(Debug)]
pub struct SkinnedMesh {
    /// 공유 메쉬 데이터입니다.
    pub(super) model_mesh: Arc<ModelMesh>, 

    /// 최상위 뼈 노드입니다.
    pub(super) root_bone: WorldID, 

    /// 스키닝 메쉬를 구성하는 뼈 노드입니다.
    pub(super) bones: Vec<WorldID>, 

    /// 메쉬의 유니폼 버퍼입니다.
    pub(super) skinned_mesh_uniform: SkinnedMeshUniform, 

    /// 메쉬의 바인드 포즈 유니폼 버퍼입니다.
    pub(super) bindpose_uniform: BoneUniform, 

    /// 메쉬의 뼈 변환 행렬 유니폼 버퍼입니다.
    pub(super) bone_transforms_uniform: BoneUniform, 

    /// 메쉬의 바인드 그룹입니다.
    pub(super) bind_group: wgpu::BindGroup, 
}

impl SkinnedMesh {
    /// 스키닝된 메쉬의 [wgpu::BindGroupLayout]을 가져옵니다.
    #[must_use]
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(SkinnedMesh)"), 
                    entries: &[
                        // 0번 바인딩: 스키닝된 유니폼 버퍼
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
                        // 1번 바인딩: 바인드 포즈 행렬 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None
                            },
                            count: None, 
                        },
                        // 2번 바인딩: 현재 뼈 변환 행렬 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
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

impl SkinnedMesh {
    #[inline]
    #[must_use]
    pub fn root_bone(&self) -> &WorldID {
        &self.root_bone
    }

    #[inline]
    #[must_use]
    pub fn bones(&self) -> &[WorldID] {
        &self.bones
    }

    /// 모델 메쉬 데이터를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn model_mesh(&self) -> &Arc<ModelMesh> {
        &self.model_mesh
    }
    
    /// 메쉬의 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn skinned_mesh_uniform(&self) -> &SkinnedMeshUniform {
        &self.skinned_mesh_uniform
    }

    /// 메쉬의 바인드 포즈 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bindpose_uniform(&self) -> &BoneUniform {
        &self.bindpose_uniform
    }

    /// 메쉬의 뼈 변환 행렬 유니폼 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bone_transforms_uniform(&self) -> &BoneUniform {
        &self.bone_transforms_uniform
    }

    /// 메쉬의 바인드 그룹을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
