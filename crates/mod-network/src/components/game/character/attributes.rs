//! 캐릭터 속성과 관련된 코드를 관리합니다.
//!

use mod_physics::object3d::Capsule;
use serde::{Deserialize, Serialize};

use crate::components::{Float3, Float4x4};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeaponAttributes {
    pub bip001: Float4x4,
    pub bip001_pelvis: Float4x4,
    pub bip001_spine: Float4x4,
    pub bip001_spine1: Float4x4,
    pub bip001_clavicle: Float4x4,
    pub bip001_upperarm: Float4x4,
    pub bip001_forearm: Float4x4,
    pub bip001_hand: Float4x4,
    pub hand_to_weapon_offset: Float4x4,
    pub bip001_fire: Float4x4,
}

/// 캐릭터 속성 데이터를 저장합니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CharacterAttributes {
    /// 캐릭터 이동 속도
    pub speed: f32,

    /// 왼쪽 무기 속성 데이터
    pub left_weapon: Option<WeaponAttributes>,
    /// 오른쪽 무기 속성 데이터
    pub right_weapon: Option<WeaponAttributes>,

    /// 일반 공격 상태 일 때 `Head` 뼈 노드의 축
    pub attack_head_axis: Float3,
    /// 일반 공격 상태 일 때 `Spine` 뼈 노드의 축
    pub attack_spine_axis: Float3,
    /// 일반 공격 상태 일 때 `Spine1` 뼈 노드의 축
    pub attack_spine1_axis: Float3,

    /// 스킬 시전 상태 일 때 `Head` 뼈 노드의 축
    pub skill_head_axis: Float3,
    /// 스킬 시전 상태 일 때 `Spine` 뼈 노드의 축
    pub skill_spine_axis: Float3,
    /// 스킬 시전 상태 일 때 `Spine1` 뼈 노드의 축
    pub skill_spine1_axis: Float3,

    /// `ActionState::Idle` 애니메이션 시간 (단위: ms)
    pub normal_idle_duration: u16,
    /// 걷기 애니메이션 시간 (단위: ms)
    pub cafe_walk_duration: u16,
    /// `MovementState::Moving` 애니메이션 시간 (단위: ms)
    pub move_ing_duration: u16,
    /// `MovementState::MoveToEnd` 애니메이션 시간 (단위: ms)
    pub move_end_normal_duration: u16,
    /// `ActionState::Aiming` 애니메이션 시간 (단위: ms)
    pub normal_attack_start_duration: u16,
    /// `ActionState::AimOff` 애니메이션 시간 (단위: ms)
    pub normal_attack_end_duration: u16,
    /// `ActionState::Attack` 애니메이션 시간 (단위: ms)
    pub normal_attack_ing_duration: u16,
    /// `ActionState::Dead` 애니메이션 시간 (단위: ms)
    pub vital_death_duration: u16,
    /// `ActionState::Reload` 애니메이션 시간 (단위: ms)
    pub normal_reload_duration: u16,
    /// `ActionState::Skill` 애니메이션 시간 (단위: ms)
    pub skill_duration: u16,
    /// `AcstionState::Callsign` 애니메이션 시간 (단위: ms)
    pub normal_callsign_duration: u16,
    /// `ActionState::VictoryStart` 애니메이션 시간 (단위: ms)
    pub victory_start_duration: u16,
    /// `ActionState::VictoryStart` 애니메이션 시간 (단위: ms)
    pub victory_end_duration: u16,

    /// 일반 공격 총알 발사 시간 (단위: ms)
    pub normal_attack_timing: Vec<u16>,
    /// 일반 공격 총알 발사 수
    pub normal_attack_count: u16,
    /// 총알의 최대 개수
    pub max_bullets: u16,
    /// 캐릭터의 최대 체력
    pub max_health_point: u16,
    /// 캐릭터의 공격력
    pub attack_power: u16,
    /// 캐릭터의 방어력
    pub defense_power: u16,
    /// 캐릭터의 명중 수치
    pub accuracy_stat: u16,
    /// 캐릭터의 회피 수치
    pub evasion_stat: u16,
    /// 캐릭터의 치명 수치
    pub critical_rate: u16,
    /// 캐릭터의 치명 데미지
    pub critical_damage: u16,
    /// 캐릭터의 최대 스킬 코스트
    pub max_skill_cost: u16,
    /// 캐릭터의 사용 스킬 코스트
    pub skill_cost: u16,
    /// 캐릭터의 공격 사거리
    pub attack_range: u16,
    /// 캐릭터 총알의 반지름
    pub bullet_radius: f32,

    /// 캐릭터 충돌체
    pub collider: Capsule,
}
