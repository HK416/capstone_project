//! 메쉬 쉐이더 리소스와 관련된 코드를 관리합니다.
//!
use std::sync::{Arc, OnceLock};

use crate::component::{BindposeUniform, BoneTransformUniform, SkinningUniform, TransformUniform};

/// 메쉬 쉐이더 리소스입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshResource(Arc<wgpu::BindGroup>);

impl MeshResource {
    /// 쉐이더 리소스의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(MeshResource)"),
                entries: &[
                    // 0번 바인딩: 변환 행렬 유니폼 버퍼
                    TransformUniform::bind_group_layout_entry(0),
                ],
            })
        })
    }

    /// 쉐이더 리소스를 생성합니다.
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        transform_uniform: &TransformUniform,
    ) -> Self {
        Self(Arc::new(device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
                layout: Self::bind_group_layout(device),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: transform_uniform.as_entire_binding(),
                }],
            },
        )))
    }

    /// [wgpu::BindGroup]을 반환합니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.0
    }
}

/// 스키닝된 메쉬 쉐이더 리소스입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinnedMeshResource(Arc<wgpu::BindGroup>);

impl SkinnedMeshResource {
    /// 쉐이더 리소스의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(SkinnedMeshResource)"),
                entries: &[
                    // 0번 바인딩: 스키닝 데이터 유니폼 버퍼
                    SkinningUniform::bind_group_layout_entry(0),
                    // 1번 바인딩: 바인드 포즈 유니폼 버퍼
                    BindposeUniform::bind_group_layout_entry(1),
                    // 2번 바인딩: 뼈 변환 행렬 유니폼 버퍼
                    BoneTransformUniform::bind_group_layout_entry(2),
                ],
            })
        })
    }

    /// 쉐이더 리소스를 생성합니다.
    pub fn new(
        label: Option<&str>,
        device: &wgpu::Device,
        skinning_uniform: &SkinningUniform,
        bindpose_uniform: &BindposeUniform,
        bone_trans_uniform: &BoneTransformUniform,
    ) -> Self {
        Self(Arc::new(device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
                layout: Self::bind_group_layout(device),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: skinning_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: bindpose_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: bone_trans_uniform.as_entire_binding(),
                    },
                ],
            },
        )))
    }

    /// [wgpu::BindGroup]을 반환합니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.0
    }
}
