use glam::Vec4Swizzles;
use hecs::{Entity, World};
use mod_network::components::{ViewState, ViewStateTimer};

use super::{update_entity_hierarchy, ToParentTrans};

/// ## Third Person Camera Data
#[derive(Debug, Clone, Copy)]
pub struct ThirdPersonCamera {
    /// 삼인칭 카메라의 기본 위치 오프셋입니다.
    pub default_offset: glam::Vec4,

    /// 삼인칭 카메라의 줌 위치 오프셋입니다.
    pub zoom_offset: glam::Vec4,

    /// 삼인칭 카메라의 위치 오프셋입니다.
    pub position_offset: glam::Vec4,

    /// 카메라가 대상을 바라보는 방향입니다.
    pub yaw_angle: f32,

    /// 카메라가 대상을 바라보는 각도입니다.
    pub pitch_angle: f32,
}

impl ThirdPersonCamera {
    /// 삼인칭 카메라가 바라보는 방향 회전시킵니다.
    pub fn rotate(&mut self, dx: f32, dy: f32, offset: f32) {
        use core::f32::consts::{FRAC_PI_3, TAU};

        // 삼인칭 카메라가 바라보는 방향을 갱신합니다.
        let angle = (dx * offset).to_radians();
        self.yaw_angle = (self.yaw_angle + angle) % TAU;

        // 삼인칭 카메라의 바라보는 각도를 갱신합니다.
        let angle = (dy * offset).to_radians();
        self.pitch_angle = (self.pitch_angle + angle).clamp(-FRAC_PI_3, FRAC_PI_3);
    }

    /// 현재 삼인칭 카메라의 위치 오프셋을 갱신합니다.
    ///
    /// # Note
    /// 이 함수를 호출하기 전에 `ViewState`가 먼저 갱신되어야 합니다.
    ///
    pub fn update_offset(&mut self, view_state: ViewState, view_state_timer: ViewStateTimer) {
        const FUNC_TABLE: [fn(glam::Vec4, glam::Vec4, f32) -> glam::Vec4; 4] = [
            update_offset_when_idle_state,
            update_offset_when_zoom_in_state,
            update_offset_when_zoom_out_state,
            update_offset_when_aiming_state,
        ];

        let i = view_state as usize;
        let s = view_state_timer.normalize();
        self.position_offset = FUNC_TABLE[i](self.default_offset, self.zoom_offset, s);
    }

    /// 카메라의 바라보는 방향을 행렬로 반환합니다.
    pub fn to_matrix(&self) -> glam::Mat4 {
        let distance = self.position_offset.z;
        let mut transform = glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, -distance));
        let rotation = glam::Mat4::from_rotation_y(self.yaw_angle);
        transform = rotation * transform;

        let z_axis = transform.z_axis.xyz().normalize_or(glam::Vec3::Z);
        let x_axis = glam::Vec3::Y.cross(z_axis);
        let rotation = glam::Mat4::from_axis_angle(x_axis, self.pitch_angle);
        transform = rotation * transform;

        let offset = self.position_offset.xyw();
        let offset_mat = glam::Mat4::from_translation(offset);
        transform = transform * offset_mat;

        transform
    }
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self {
            default_offset: glam::Vec4::new(0.25, 0.85, 1.5, 0.0),
            zoom_offset: glam::Vec4::new(0.2, 0.6, 0.7, 0.0),
            position_offset: glam::Vec4::new(0.25, 0.85, 1.5, 0.0),
            yaw_angle: 0.0f32.to_radians(),
            pitch_angle: 10f32.to_radians(),
        }
    }
}

/// `ViewState::Idle`일 때 삼인칭 카메라의 위치 오프셋을 계산합니다.
fn update_offset_when_idle_state(default_offset: glam::Vec4, _: glam::Vec4, _: f32) -> glam::Vec4 {
    default_offset
}

/// `ViewState::ZoomIn`일 때 삼인칭 카메라의 위치 오프셋을 계산합니다.
fn update_offset_when_zoom_in_state(
    default_offset: glam::Vec4,
    zoom_offset: glam::Vec4,
    s: f32,
) -> glam::Vec4 {
    default_offset.lerp(zoom_offset, s)
}

/// `ViewState::ZoomOut`일 때 삼인칭 카메라의 위치 오프셋을 계산합니다.
fn update_offset_when_zoom_out_state(
    default_offset: glam::Vec4,
    zoom_offset: glam::Vec4,
    s: f32,
) -> glam::Vec4 {
    zoom_offset.lerp(default_offset, s)
}

/// `ViewState::Aiming`일 때 삼인칭 카메라의 위치 오프셋을 계산합니다.
fn update_offset_when_aiming_state(_: glam::Vec4, zoom_offset: glam::Vec4, _: f32) -> glam::Vec4 {
    zoom_offset
}

/// ## Projection Transform Matrix
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection(pub glam::Mat4);

impl Projection {
    /// 새로운 원근 투영 변환 행렬을 생성합니다.
    pub fn perspective(fov_y_radians: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Self {
        Self(glam::Mat4::perspective_lh(
            fov_y_radians,
            aspect_ratio,
            z_near,
            z_far,
        ))
    }
}

/// 3인칭 카메라의 월드 변환 행렬을 계산합니다.
///
/// 주어진 대상의 위치를 기준으로 카메라의 월드 변환 행렬이 계산됩니다.
///
/// # Panics
/// - 주어진 카메라 엔터티는 유효해야합니다. 그렇지 않은 경우 [`panic!`]을 호출합니다.
/// - 주어진 카메라 엔터티는 로컬 변환 행렬(`ToParentTrans`), 월드 변환 행렬(`WorldTransform`),
/// 삼인칭 카메라 요소(`ThirdPersonCamera`)를 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn update_third_person_camera_hierarchy(
    world: &mut World,
    camera_entity: Entity,
    target_position: glam::Vec4,
) {
    // 부모 변환 행렬을 생성합니다.
    let parent_transform = glam::Mat4::from_translation(target_position.xyz());

    // 카메라 엔터티의 로컬 변환 행렬을 갱신합니다.
    let (local_transform, third_person_camera) = world
        .query_one_mut::<(&mut ToParentTrans, &ThirdPersonCamera)>(camera_entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = third_person_camera.to_matrix();

    // 카메라의 월드 변환 행렬을 갱신합니다.
    update_entity_hierarchy(world, camera_entity, parent_transform);
}
