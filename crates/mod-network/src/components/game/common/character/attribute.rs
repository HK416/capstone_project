use serde::{Deserialize, Serialize};

use crate::components::{BigEndian, Float3};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum SkillKind {
    Active(f32),
    Passive,
}

impl BigEndian for SkillKind {
    fn byte_size() -> usize {
        f32::byte_size()
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // SkillKind::Active인 경우 최상위 비트를 1로 설정합니다.
        match self {
            SkillKind::Active(cool_time) => {
                let mut bitfield = cool_time.to_big_endian_bytes();
                bitfield[0] |= 0b1000_0000;
                bitfield
            }
            SkillKind::Passive => vec![0; 4],
        }
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        if bytes[0] & 0b1000_0000 == 0b1000_0000 {
            let mut bytes = bytes.to_vec();
            bytes[0] &= 0b0111_1111;
            SkillKind::Active(f32::from_big_endian_bytes(&bytes))
        } else {
            SkillKind::Passive
        }
    }
}

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
    pub normal_attack_count: u16,
    /// 총알의 최대 개수
    pub max_bullets: u16,
    /// 캐릭터의 최대 체력
    pub health_point: u16,
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
    /// 캐릭터의 코스트 회복력
    pub cost_recovery_rate: f32,
    /// 일반 스킬 쿨 타임
    pub skill_cool_time: SkillKind,
    /// 캐릭터의 공격 사거리
    pub attack_range: u16,
    /// 캐릭터 총알의 반지름
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

#[cfg(test)]
mod tests {
    use crate::components::BigEndian;

    use super::SkillKind;

    #[test]
    fn test_skill_kind() {
        let origin = SkillKind::Active(12.24132);
        let bytes = origin.to_big_endian_bytes();
        let other = SkillKind::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
