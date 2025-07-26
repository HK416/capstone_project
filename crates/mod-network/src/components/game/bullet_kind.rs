//! 총알 종류와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 총알 모델의 개수입니다.
pub const NUM_BULLETS: usize = 4;

/// 총알 모델 종류 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BulletKind {
    /// 일반 총알 모델
    #[default]
    Common = 0,
    /// 에너지 볼 형태의 총알 모델
    EnergyBoll = 1,
    /// Aris Original Skill 총알 모델
    ArisOriginalSkill = 2,
    /// Momoi Original Skill 총알 모델
    MomoiOriginalSkill = 3,
}

impl BulletKind {
    /// 주어진 정수로 `BulletKind`를 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Common),
            1 => Some(Self::EnergyBoll),
            2 => Some(Self::ArisOriginalSkill),
            3 => Some(Self::MomoiOriginalSkill),
            _ => None,
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            BulletKind::Common => 200.0,
            BulletKind::EnergyBoll => 150.0,
            BulletKind::ArisOriginalSkill => 150.0,
            BulletKind::MomoiOriginalSkill => 200.0,
        }
    }
}

impl BigEndian for BulletKind {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for BulletKind {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_bullet_kind() {
        BulletKind::new(123).unwrap();
    }

    #[test]
    fn test_bullet_kind_common() {
        let val = BulletKind::Common as u8;
        let kind = BulletKind::new(val).unwrap();
        assert_eq!(BulletKind::Common, kind);
    }

    #[test]
    fn test_bullet_kind_energy_boll() {
        let val = BulletKind::EnergyBoll as u8;
        let kind = BulletKind::new(val).unwrap();
        assert_eq!(BulletKind::EnergyBoll, kind);
    }

    #[test]
    fn test_bullet_kind() {
        let origin = BulletKind::EnergyBoll;
        let bytes = origin.to_big_endian_bytes();
        let other = BulletKind::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인한다.
        assert_eq!(origin, other);
    }
}
