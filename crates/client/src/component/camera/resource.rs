//! 카메라 쉐이더 리소스와 관련된 코드를 관리합니다.
//!

use std::sync::{Arc, OnceLock};

use crate::component::CameraUniform;

//// 카메라 쉐이더 리소스입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraResource(Arc<wgpu::BindGroup>);

impl CameraResource {
    /// [wgpu::BindGroupLayout]을 반환합니다.
    pub fn bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(CameraResource)"),
                entries: &[
                    // 0번 바인딩: 카메라 데이터 유니폼 버퍼
                    CameraUniform::bind_group_layout_entry(wgpu::ShaderStages::VERTEX_FRAGMENT, 0),
                ],
            })
        })
    }

    /// 새로운 쉐이더 리소스를 생성합니다.
    pub fn new(label: Option<&str>, device: &wgpu::Device, camera_uniform: &CameraUniform) -> Self {
        Self(Arc::new(device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", label.unwrap_or("Unknown"))),
                layout: Self::bind_group_layout(device),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform.as_entire_binding(),
                }],
            },
        )))
    }

    /// [wgpu::BindGroup]을 반환합니다.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.0
    }
}
