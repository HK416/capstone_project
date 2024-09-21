use std::{collections::HashMap, sync::{Arc, OnceLock}};

use super::{Attribute, BoneUniform, IndexBuffer, SkinnedMeshUniform, VertexBuffer};



/// 스키닝된 메쉬 데이터입니다.
#[derive(Debug)]
pub struct SkinnedMesh {
    /// 메쉬의 이름입니다.
    pub(super) name: String, 

    /// 메쉬의 정점 개수입니다.
    pub(super) num_vertices: u32, 

    /// 메쉬의 정점 버퍼입니다.
    pub(super) vertex: VertexBuffer, 

    /// 메쉬의 정점 속성 버퍼입니다.
    pub(super) attributes: HashMap<Attribute, VertexBuffer>, 
    
    /// 메쉬의 하위 메쉬들입니다.
    pub(super) submeshes: Vec<IndexBuffer>, 
    
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
    pub fn bind_group_layout(device: &Arc<wgpu::Device>) -> &'static wgpu::BindGroupLayout {
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
    /// 메쉬의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 정점의 개수를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn num_vertices(&self) -> u32 {
        self.num_vertices
    }

    /// 메쉬의 정점 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn vertex(&self) -> &VertexBuffer {
        &self.vertex
    }

    /// 메쉬의 정점 속성 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn attribute(&self, id: &Attribute) -> Option<&VertexBuffer> {
        self.attributes.get(id)
    }

    /// 메쉬의 하위 메쉬들을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn submeshes(&self) -> &[IndexBuffer] {
        &self.submeshes
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
