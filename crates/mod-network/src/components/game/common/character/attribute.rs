use serde::{Deserialize, Serialize};

use crate::components::Float3;

/// 캐릭터 속성 데이터를 저장합니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharacterAttributes {
    /// 캐릭터 이동 속도
    pub speed: f32,
    /// 위도가 최소(-60도)이고, `ActionState::Aim`일 때 총구의 상대 위치
    pub muzzle_position_min: Float3,
    /// 위도가 최소(-60도)이고, `ActionState::Aim`일 때 총구가 향하는 방향
    pub muzzle_direction_min: Float3,
    /// 위도가 0도이고, `ActionState::Aim`일 때 총구의 상대 위치
    pub muzzle_position_mid: Float3,
    /// 위도가 0도이고, `ActionState::Aim`일 때 총구가 향하는 방향
    pub muzzle_direction_mid: Float3,
    /// 위도가 최대(60도)이고, `ActionState::Aim`일 때 총구의 상대 위치
    pub muzzle_position_max: Float3,
    /// 위도가 최대(60도)이고, `ActionState::Aim`일 때 총구가 향하는 방향
    pub muzzle_direction_max: Float3,
    /// `MovementState::Moving` 애니메이션 시간 (단위: 초)
    pub move_ing_duration: f32,
    /// `MovementState::MoveToEnd` 애니메이션 시간 (단위: 초)
    pub move_end_normal_duration: f32,
    /// 걷기 애니메이션 시간 (단위: 초)
    pub walk_duration: f32,
    /// `ActionState::Idle` 애니메이션 시간 (단위: 초)
    pub normal_idle_duration: f32,
    /// `ActionState::Reload` 애니메이션 시간 (단위: 초)
    pub normal_reload_duration: f32,
    /// `ActionState::Aiming` 애니메이션 시간 (단위: 초)
    pub normal_attack_start_duration: f32,
    /// `ActionState::AimOff` 애니메이션 시간 (단위: 초)
    pub normal_attack_end_duration: f32,
    /// `ActionState::Attack` 애니메이션 시간 (단위: 초)
    pub normal_attack_ing_duration: f32,
    /// 일반 공격 총알 발사 시간 (단위: 초)
    pub normal_attack_timing: Vec<f32>,
    /// 일반 공격 총알 발사 수
    pub normal_attack_count: u32,
    /// 총알의 최대 개수
    pub max_bullets: u32,
    pub health_point: u32,
    pub attack_power: u32,
    pub defense_power: u32,
    pub accuracy_stat: u32,
    pub evasion_stat: u32,
    pub critical_rate: u32,
    pub critical_damage: u32,
    pub attack_range: u32,
    pub bullet_radius: f32,
}

impl CharacterAttributes {
    /// 라그랑주 보간법을 사용하여 총구의 위치를 계산합니다.
    ///
    /// # Note
    /// t의 값은 0부터 1사이의 값 입니다.
    ///
    pub fn get_muzzle_position(&self, t: f32) -> (f32, f32, f32) {
        let l1 = ((t - 0.5) * (t - 1.0)) / 0.5;
        let l2 = (t * (t - 1.0)) / -0.25;
        let l3 = (t * (t - 0.5)) / 0.5;

        let (x1, y1, z1): (f32, f32, f32) = self.muzzle_position_min.into();
        let (x2, y2, z2): (f32, f32, f32) = self.muzzle_position_mid.into();
        let (x3, y3, z3): (f32, f32, f32) = self.muzzle_position_max.into();

        let x = x1 * l1 + x2 * l2 + x3 * l3;
        let y = y1 * l1 + y2 * l2 + y3 * l3;
        let z = z1 * l1 + z2 * l2 + z3 * l3;

        (x, y, z)
    }

    /// 라그랑주 보간법을 사용하여 총구의 방향을 계산합니다.
    ///
    /// # Note
    /// t의 값은 0부터 1사이의 값 입니다.
    ///
    pub fn get_muzzle_direction(&self, t: f32) -> (f32, f32, f32) {
        let l1 = ((t - 0.5) * (t - 1.0)) / 0.5;
        let l2 = (t * (t - 1.0)) / -0.25;
        let l3 = (t * (t - 0.5)) / 0.5;

        let (x1, y1, z1): (f32, f32, f32) = self.muzzle_direction_min.into();
        let (x2, y2, z2): (f32, f32, f32) = self.muzzle_direction_mid.into();
        let (x3, y3, z3): (f32, f32, f32) = self.muzzle_direction_max.into();

        let x = x1 * l1 + x2 * l2 + x3 * l3;
        let y = y1 * l1 + y2 * l2 + y3 * l3;
        let z = z1 * l1 + z2 * l2 + z3 * l3;

        (x, y, z)
    }
}
