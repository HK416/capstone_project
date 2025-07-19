//! 캐릭터 위치 갱신과 관련된 코드를 관리합니다.
//!

use std::f32::{consts::PI, EPSILON};

use mod_physics::{
    collision::{Collider, ColliderTreeIterator},
    object3d::BoundingBox,
};

use crate::components::{
    ActionState, CharacterAttributes, HealthData, HeldInput, InputStateTimer, LatLon,
    MovementState, MovementStateTimer, MovingDirection, StageAttributes, Team, Velocity,
};

/// 중력 가속도 (단위: m/s^2)
pub const GRAVITY: f32 = -9.80665;
/// 임계 각도 (단위: 라디안)
const GROUNDED_ANGLE: f32 = 45f32.to_radians();

/// 플레이어 방향을 갱신합니다.
pub fn update_player_rotation(
    look: glam::Vec3A,
    action_state: ActionState,
    movement_state: MovementState,
    direction: MovingDirection,
    latlon: LatLon,
) -> glam::Vec3A {
    match action_state {
        ActionState::Idle => match movement_state {
            MovementState::Idle
            | MovementState::MoveToEnd
            | MovementState::Jumping
            | MovementState::Landing => update_rotation_when_none(look, direction, latlon),
            MovementState::Moving => update_rotation_when_to_direction(look, direction, latlon),
        },
        ActionState::AimAt => match movement_state {
            MovementState::Idle
            | MovementState::Moving
            | MovementState::Jumping
            | MovementState::Landing => update_rotation_when_to_camera(look, direction, latlon),
            MovementState::MoveToEnd => update_rotation_when_none(look, direction, latlon),
        },
        ActionState::AimOff => match movement_state {
            MovementState::Moving => update_rotation_when_to_direction(look, direction, latlon),
            MovementState::Idle
            | MovementState::MoveToEnd
            | MovementState::Jumping
            | MovementState::Landing => update_rotation_when_none(look, direction, latlon),
        },
        ActionState::Aiming | ActionState::Attack | ActionState::Reload | ActionState::Skill => {
            update_rotation_when_to_camera(look, direction, latlon)
        }
        ActionState::Death
        | ActionState::Callsign
        | ActionState::VictoryStart
        | ActionState::VictoryEnd => update_rotation_when_none(look, direction, latlon),
    }
}

/// 캐릭터 방향을 반환합니다.
fn update_rotation_when_none(
    look: glam::Vec3A,
    _direction: MovingDirection,
    _latlon: LatLon,
) -> glam::Vec3A {
    look
}

/// 캐릭터 방향을 카메라 방향으로 변환합니다.
fn update_rotation_when_to_camera(
    look: glam::Vec3A,
    _direction: MovingDirection,
    latlon: LatLon,
) -> glam::Vec3A {
    // 카메라 방향을 계산합니다.
    let angle = latlon.lon;
    let matrix = glam::Mat4::from_rotation_y(angle);
    let cam_look = matrix.transform_vector3a(glam::Vec3A::Z);

    // 두 벡터의 각도를 계산합니다.
    let angle = look.angle_between(cam_look);
    if (angle - PI) <= EPSILON {
        cam_look
    } else {
        // 보간된 방향을 계산합니다.
        look.lerp(cam_look, 0.2).normalize_or(cam_look)
    }
}

/// 캐릭터 방향을 움직임 방향으로 변환합니다.
fn update_rotation_when_to_direction(
    look: glam::Vec3A,
    direction: MovingDirection,
    _latlon: LatLon,
) -> glam::Vec3A {
    // 두 벡터의 각도를 계산합니다.
    let angle = look.angle_between(direction.0);
    if (angle - PI).abs() <= EPSILON {
        direction.0
    } else {
        // 보간된 방향을 계산합니다.
        look.lerp(direction.0, 0.2).normalize_or(direction.0)
    }
}

/// 플레이어 위치를 갱신합니다.
pub fn update_player_translation(
    stage_attributes: &StageAttributes,
    character_attributes: &CharacterAttributes,
    action_state: ActionState,
    movement_state: &mut MovementState,
    movement_state_timer: &mut MovementStateTimer,
    velocity: &mut Velocity,
    translation: &mut glam::Vec3A,
    direction: MovingDirection,
    held_input: HeldInput,
    team: Team,
    is_grounded: &mut bool,
    is_invincible: &mut bool,
    health_data: Option<&mut HealthData>,
    input_state_timer: InputStateTimer,
    elapsed_time_sec: f32,
) {
    // 플레이어 위치를 가져옵니다.
    let old_p = translation.clone();

    // 플레이어 속도를 갱신합니다.
    velocity.update(
        direction,
        input_state_timer,
        action_state,
        *movement_state,
        *movement_state_timer,
        character_attributes,
    );

    // 플레이어 이동 속도를 가져옵니다.
    let mut new_vel = velocity.0.clone();

    // 중력 가속도를 적용합니다.
    if !*is_grounded {
        new_vel.y += GRAVITY * elapsed_time_sec;
    }

    // 이동 시도 (이동 전 위치 저장)
    let mut new_p = old_p + new_vel * elapsed_time_sec;

    // 충돌 처리 시작
    *is_grounded = false;

    let mut player_capsule = character_attributes.collider.clone();
    player_capsule.center = new_p.into();
    let player_aabb = BoundingBox::from(&player_capsule);
    let player_collider = Collider::Capsule(player_capsule);

    for collider in ColliderTreeIterator::new(&stage_attributes.collider) {
        if !collider.check_aabb_collision(&player_aabb) {
            continue;
        }

        if let Some(details) = player_collider.check_collision_details(collider) {
            new_p += details.normal * details.penetration;

            // 충돌 벡터가 지면(xz평면)과 일정 이상의 각을 이루면 서있을 수 있음
            if details.normal.y >= GROUNDED_ANGLE.cos() {
                new_vel.y = 0.0;
                *is_grounded = true;
            }
            // 아니라면 미끄러지도록 처리
            else {
                let slide = new_vel - details.normal * new_vel.dot(details.normal);
                // +y 방향으로 튀어오르지 않게 한다.
                let vy = if slide.y < new_vel.y {
                    slide.y
                } else {
                    new_vel.y
                };
                new_vel = glam::Vec3A::new(slide.x, vy, slide.z);
            }
        }
    }

    // 유효한 위치인지 확인합니다.
    if !stage_attributes.is_valid_position(team, new_p.x, new_p.z) {
        let (x, z) = stage_attributes.get_nearest_valid_position(new_p.x, new_p.z);
        if (x - new_p.x).abs() <= EPSILON {
            new_vel.x = 0.0;
            new_p.x = x;
        }
        if (z - new_p.z).abs() <= EPSILON {
            new_vel.z = 0.0;
            new_p.z = z;
        }
    }

    // 지형의 높이를 확인합니다.
    if let Some(height) = stage_attributes.get_area_height(new_p.x, new_p.z) {
        if height >= new_p.y {
            new_p.y = height;
            new_vel.y = 0.0;
            *is_grounded = true;
        }
    }

    // 플레이어가 안전 구역 안에 있는 경우
    let in_safe_area = stage_attributes.is_safe_area(team, new_p.x, new_p.z);
    *is_invincible = in_safe_area;
    if let Some(health_data) = health_data
        && in_safe_area
    {
        /// 초당 회복량
        const HEALING: f32 = 500.0 / 1000.0;
        let healing = (HEALING * elapsed_time_sec).floor() as u16;
        health_data.remaining =
            (health_data.remaining.saturating_add(healing)).min(health_data.num_maximum_health());
    }

    if *is_grounded {
        match movement_state {
            MovementState::Landing => {
                if held_input.contains(HeldInput::Jump) {
                    *movement_state = MovementState::Jumping;
                    movement_state_timer.0 = 0;
                } else if held_input.is_moved() {
                    *movement_state = MovementState::Moving;
                    movement_state_timer.0 = 0;
                } else {
                    *movement_state = MovementState::Idle;
                    movement_state_timer.0 = 0;
                }
            }
            MovementState::Jumping => {
                new_vel.y = 5.0;
            }
            _ => {}
        }
    }

    velocity.0 = new_vel;
    *translation = new_p;
}
