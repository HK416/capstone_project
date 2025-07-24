//! 게임 진행 단계와 관련된 코드를 관리합니다.
//!

mod bullet;
mod capture_point;
mod damage;
mod player;
mod snapshot;

use crate::components::{CharacterAttributes, LatLon, ViewState, ViewStateTimer};

pub use self::{bullet::*, capture_point::*, damage::*, player::*, snapshot::*};

/// 게임에 참여 가능한 최대 플레이어 수 입니다.
pub const MAX_IN_GAME_PLAYERS: usize = 10;
static_assertions::const_assert!(MAX_IN_GAME_PLAYERS > 0);

/// 게임에 존재 가능한 최대 총알의 수 입니다.
pub const MAX_IN_GAME_BULLETS: usize = u16::MAX as usize;
static_assertions::const_assert!(MAX_IN_GAME_PLAYERS > 0);

/// 게임에 참여 가능한 한 팀당 최대 플레이어 수 입니다.
pub const MAX_IN_GAME_TEAM_PLAYERS: usize = MAX_IN_GAME_PLAYERS / 2;
static_assertions::const_assert!(MAX_IN_GAME_TEAM_PLAYERS > 0);

/// 카메라 변환 행렬을 반환합니다.
pub fn get_camera_transform(
    view_state: ViewState,
    view_state_timer: ViewStateTimer,
    character_attributes: &CharacterAttributes,
    latlon: LatLon,
) -> glam::Mat4 {
    let camera_default_pos: glam::Vec3A = character_attributes.camera_def_rel_pos.into();
    let camera_zoom_pos: glam::Vec3A = character_attributes.camera_zoom_rel_pos.into();
    let camera_rel_pos = match view_state {
        ViewState::Idle => camera_default_pos,
        ViewState::ZoomIn => {
            let duration = character_attributes.normal_attack_start_duration;
            let s = view_state_timer.0 as f32 / duration as f32;
            camera_default_pos.lerp(camera_zoom_pos, s)
        }
        ViewState::ZoomOut => {
            let duration = character_attributes.normal_attack_end_duration;
            let s = view_state_timer.0 as f32 / duration as f32;
            camera_zoom_pos.lerp(camera_default_pos, s)
        }
        ViewState::Aiming => camera_zoom_pos,
    };

    let distance = camera_rel_pos * glam::Vec3A::NEG_Z;
    let mut transform = glam::Mat4::from_translation(distance.into());
    let rotate = glam::Mat4::from_rotation_y(latlon.lon);
    transform = rotate * transform;

    let forward = glam::Vec3A::from_vec4(transform.z_axis);
    let forward = forward.normalize_or(glam::Vec3A::Z);
    let axis = glam::Vec3A::Y.cross(forward);
    let rotate = glam::Mat4::from_axis_angle(axis.into(), latlon.lat);
    transform = rotate * transform;

    let offset = camera_rel_pos.with_z(0.0);
    let offset = glam::Mat4::from_translation(offset.into());
    transform = transform * offset;

    transform
}
