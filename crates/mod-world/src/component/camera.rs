use std::sync::OnceLock;

use crate::render::camera::CameraUniform;

use super::GameObject;



/// 카메라의 [wgpu::BindGroupLayout]을 가져옵니다.
#[inline]
#[must_use]
pub fn camera_bind_group_layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
    static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
    LAYOUT.get_or_init(|| {
        device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout(CameraObject)"), 
                entries: &[
                    // 0번 바인딩: 카메라 유니폼
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, 
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, 
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Uniform, 
                            has_dynamic_offset: false, 
                            min_binding_size: None 
                        }, 
                        count: None,
                    }, 
                    // 1번 바인딩: 카메라 유니폼
                    wgpu::BindGroupLayoutEntry {
                        binding: 1, 
                        visibility: wgpu::ShaderStages::FRAGMENT, 
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Uniform, 
                            has_dynamic_offset: false, 
                            min_binding_size: None 
                        }, 
                        count: None,
                    }, 
                    // 2번 바인딩: 카메라 유니폼
                    wgpu::BindGroupLayoutEntry {
                        binding: 2, 
                        visibility: wgpu::ShaderStages::FRAGMENT, 
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



/// 게임 세상에 존재하는 카메라 오브젝트가 구현해야하는 `trait`입니다.
pub trait CameraObject : GameObject {
    /// 카메라 유니폼 버퍼를 가져옵니다.
    fn camera_uniform(&self) -> &CameraUniform;

    /// 카메라의 [wgpu::BindGroup]을 가져오빈다.
    fn bind_group(&self) -> &wgpu::BindGroup;


    /// 카메라 변환 행렬을 가져옵니다.
    fn camera_transform(&self) -> gmm::Matrix;

    /// 카메라 변환 행렬의 역행렬을 가져옵니다.
    fn inv_camera_transform(&self) -> gmm::Matrix;


    /// 투영 변환 행렬을 가져옵니다.
    fn projection_transform(&self) -> gmm::Matrix;

    /// 투영 변환 행렬의 역행렬을 가져옵니다.
    fn inv_projection_transform(&self) -> gmm::Matrix;
}

impl std::fmt::Debug for dyn CameraObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(CameraObject))
            .field("id", &self.id())
            .field("name", &self.name())
            .finish()
    }
}
