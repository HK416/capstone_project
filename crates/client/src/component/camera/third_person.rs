//! 삼인칭 카메라와 관련된 코드를 관리합니다.
//!

use hecs::{Entity, World};
use mod_network::components::{CharacterKind, LatLon};

use crate::component::{
    // create_third_person_camera_of_character,
    update_entity_hierarchy,
    ToParentTrans,
};

/// 삼인칭 카메라 요소입니다.
#[derive(Debug, Clone, Copy)]
pub struct ThirdPersonCamera {
    /// 삼인칭 카메라 Fov-y 입니다.
    pub fov_y: f32,
    /// 카메라의 회전 각도입니다.
    pub rotation: LatLon,
    /// 삼인칭 카메라 상대 위치입니다.
    pub position: glam::Vec3A,
}

impl ThirdPersonCamera {
    /// 캐릭터가 바라보는 방향으로 삼인칭 카메라를 생성합니다.
    pub fn new(character_kind: CharacterKind, rotation: LatLon) -> Self {
        // create_third_person_camera_of_character(character_kind, rotation)
        todo!()
    }

    /// 삼인칭 카메라가 바라보는 방향 회전시킵니다.
    pub fn rotate(&mut self, dx: f32, dy: f32, offset: f32) {
        // use core::f32::consts::TAU;

        // // Fov-y에 따른 카메라 이동 속도를 조절합니다.
        // const RANGE: f32 = 90f32.to_radians();
        // let speed = 0.1 + 0.9 * (self.fov_y / RANGE).min(1.0);

        // // 삼인칭 카메라가 바라보는 방향을 갱신합니다.
        // let angle = (dx * offset * speed).to_radians();
        // self.rotation.lon = (self.rotation.lon + angle) % TAU;

        // // 삼인칭 카메라의 바라보는 각도를 갱신합니다.
        // let angle = (dy * offset * speed).to_radians();
        // self.rotation.lat =
        //     (self.rotation.lat + angle).clamp(LatLon::MIN_LATITUDE, LatLon::MAX_LATITUDE);
    }

    /// 카메라의 바라보는 방향을 행렬로 반환합니다.
    pub fn to_matrix(&self) -> glam::Mat4 {
        // let distance = self.position.z;
        // let mut transform = glam::Mat4::from_translation(glam::vec3(0.0, 0.0, -distance));
        // let rotation = glam::Mat4::from_rotation_y(self.rotation.lon);
        // transform = rotation * transform;

        // let z_axis = glam::Vec3A::from_vec4(transform.z_axis).normalize_or(glam::Vec3A::Z);
        // let x_axis = glam::Vec3A::Y.cross(z_axis);
        // let rotation = glam::Mat4::from_axis_angle(x_axis.into(), self.rotation.lat);
        // transform = rotation * transform;

        // let offset = self.position.with_z(0.0);
        // let offset_mat = glam::Mat4::from_translation(offset.into());
        // transform = transform * offset_mat;

        // transform
        todo!()
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
pub fn update_third_person_camera_hierarchy<P>(world: &mut World, entity: Entity, target_pos: P)
where
    P: Into<glam::Vec3>,
{
    // 부모 변환 행렬을 생성합니다.
    let parent_transform = glam::Mat4::from_translation(target_pos.into());

    // 카메라 엔터티의 로컬 변환 행렬을 갱신합니다.
    let (local_transform, third_person_camera) = world
        .query_one_mut::<(&mut ToParentTrans, &ThirdPersonCamera)>(entity)
        .expect("invalid entity or invalid entity component");
    local_transform.0 = third_person_camera.to_matrix();

    // 카메라의 월드 변환 행렬을 갱신합니다.
    update_entity_hierarchy(world, entity, parent_transform);
}
