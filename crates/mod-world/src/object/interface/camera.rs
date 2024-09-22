use std::sync::{Arc, OnceLock};

use crate::render::camera::CameraUniform;

use super::GameObject;



/// ### Camera Object
/// 게임 세상에 존재하는 모든 카메라는 `CameraObject`를 구현해야 합니다.
/// 
pub trait CameraObject : GameObject {
    /// 카메라(뷰) 변환 행렬을 가져옵니다.
    fn camera_trans(&self) -> gmm::Matrix;

    /// 카메라(뷰) 변환 행렬의 역행렬을 가져옵니다.
    fn inv_camera_trans(&self) -> gmm::Matrix;

    /// 투영 변환 행렬을 가져옵니다.
    fn projection_trans(&self) -> gmm::Matrix;

    /// 투영 변환 행렬의 역행렬을 가져옵니다.
    fn inv_projection_trans(&self) -> gmm::Matrix;


    /// 카메라의 바인드 그룹을 가져옵니다.
    fn bind_group(&self) -> &wgpu::BindGroup;

    /// 카메라의 [wgpu::BindGroupLayout]을 가져옵니다.
    #[must_use]
    fn bind_group_layout(device: &Arc<wgpu::Device>) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(CameraObject)"), 
                    entries: &[
                        // 0번 바인딩: 카메라 유니폼 데이터
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None
                        }, 
                        // 1번 바인딩: 전역 조명 유니폼 데이터
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None
                        }, 
                        // 2번 바인딩: 지역 조명 유니폼 데이터
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
                            visibility: wgpu::ShaderStages::FRAGMENT, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None
                        }, 
                    ]
                }
            )
        })
    }

    /// 카메라 유니폼 버퍼를 가져옵니다.
    fn camera_uniform(&self) -> &CameraUniform;
}
