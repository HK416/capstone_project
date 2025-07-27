//! 조명과 관련된 코드를 관리합니다.
//!

mod resource;
mod uniform;

use std::sync::Arc;

pub use self::{resource::*, uniform::*};

use super::WorldTransform;

/// 최대 조명의 개수입니다. (최대 로컬 조명의 수)
pub const MAX_LIGHTS: usize = 8;

/// 그림자 맵 텍스처의 텍스처 포맷입니다.
pub const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// 로컬 조명의 그림자 맵 텍스처의 가로 세로 크기입니다.
pub const LOCAL_SHADOW_MAP_SIZE: u32 = 1024;

/// 전역 조명의 그림자 맵 텍스처의 가로 세로 크기입니다.
pub const GLOBAL_SHADOW_MAP_SIZE: u32 = 4096;

/// 방향 조명 데이터입니다.
#[derive(Debug, Clone)]
pub struct DirectionLight {
    /// 미리 계산된 그림자 맵 텍스처 뷰 입니다.
    pub shadow_map_view: Arc<wgpu::TextureView>,
    /// 미리 계산된 그림자 맵 텍스처 샘플러입니다.
    pub shadow_sampler: Arc<wgpu::Sampler>,
    /// 미리 계산된 그림자 맵의 조명 변환 행렬입니다.
    pub light_proj_view: glam::Mat4,
    /// 월드 좌표 상 조명이 비추는 방향입니다.
    pub direction_w: glam::Vec3A,
    /// 조명의 색깔입니다.
    pub color: glam::Vec3,
}

// /// Cascade 분할
// pub fn compute_cascade_splits(num_cascades: usize, near: f32, far: f32, lambda: f32) -> Vec<f32> {
//     (0..num_cascades)
//         .map(|i| {
//             let p = (i + 1) as f32 / num_cascades as f32;
//             let log = near * (far / near).powf(p);
//             let uni = near + (far - near) * p;
//             log * lambda + uni * (1.0 - lambda)
//         })
//         .collect()
// }

/// 카메라 공간에서 cascade 코너를 계산한 뒤, 카메라 기준으로 월드로 변환
pub fn compute_frustum_corners_no_inverse(
    transform: &WorldTransform,
    fov_y: f32,
    aspect: f32,
    near: f32,
    far: f32,
) -> [glam::Vec3A; 8] {
    let tan_fov = (fov_y * 0.5).tan();

    let h_near = 2.0 * tan_fov * near;
    let w_near = h_near * aspect;
    let h_far = 2.0 * tan_fov * far;
    let w_far = h_far * aspect;

    let cam_pos = transform.get_translation();
    let cam_right = transform.get_right_vector();
    let cam_up = transform.get_up_vector();
    let cam_forward = transform.get_look_vector();

    let center_near = cam_pos + cam_forward * near;
    let center_far = cam_pos + cam_forward * far;

    let up_near = cam_up * (h_near * 0.5);
    let right_near = cam_right * (w_near * 0.5);

    let up_far = cam_up * (h_far * 0.5);
    let right_far = cam_right * (w_far * 0.5);

    // Near plane
    let ntl = center_near + up_near - right_near;
    let ntr = center_near + up_near + right_near;
    let nbl = center_near - up_near - right_near;
    let nbr = center_near - up_near + right_near;

    // Far plane
    let ftl = center_far + up_far - right_far;
    let ftr = center_far + up_far + right_far;
    let fbl = center_far - up_far - right_far;
    let fbr = center_far - up_far + right_far;

    [nbl, nbr, ntl, ntr, fbl, fbr, ftl, ftr] // 맞춰진 순서
}

/// Light 방향으로 프러스텀의 AABB를 projection하는 ViewProj 행렬 생성
pub fn compute_light_view_proj_matrix(
    frustum_corners: &[glam::Vec3A; 8],
    light_dir: glam::Vec3A,
    margin: f32,
) -> glam::Mat4 {
    let center =
        frustum_corners.iter().copied().sum::<glam::Vec3A>() / frustum_corners.len() as f32;
    let light_pos = center - light_dir.normalize() * 100.0;
    let light_view = glam::Mat4::look_at_lh(light_pos.into(), center.into(), glam::Vec3::Y);

    let mut min = glam::Vec3A::splat(f32::MAX);
    let mut max = glam::Vec3A::splat(f32::MIN);

    for &corner in frustum_corners.iter() {
        let corner_ls = light_view.transform_point3a(corner);
        min = min.min(corner_ls);
        max = max.max(corner_ls);
    }

    min -= glam::Vec3A::splat(margin);
    max += glam::Vec3A::splat(margin);

    let light_proj = glam::Mat4::orthographic_lh(min.x, max.x, min.y, max.y, min.z, max.z + 5.0);

    light_proj * light_view
}
