//! 데미지 로그 데이터와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian, UserId};

/// 최대 데미지 량
pub const MAX_DAMAGE: u16 = 9_999;

/// 데미지 종류입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DamageKind {
    Common = 0,
    Critial = 1,
    Miss = 2,
}

impl DamageKind {
    /// 정수로 부터 `DamageKind`를 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub const fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Common),
            1 => Some(Self::Critial),
            2 => Some(Self::Miss),
            _ => None,
        }
    }
}

/// 비트 필드 데이터입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Bitfield(u16);

impl Bitfield {
    const DAMAGE_BIT_MASK: u16 = 0b11_1111_1111_1111;
    const DAMAGE_SHIFT: usize = 0;
    const KIND_BIT_MASK: u16 = 0b11;
    const KIND_SHIFT: usize = 14;

    /// 새로운 비트 필드 데이터를 생성합니다.
    pub const fn new() -> Self {
        Self(0x0000)
    }

    // 데미지 량을 반환합니다.
    pub fn damage(&self) -> u16 {
        ((self.0 >> Self::DAMAGE_SHIFT) & Self::DAMAGE_BIT_MASK).min(MAX_DAMAGE)
    }

    /// 데미지 량을 설정합니다.
    ///
    /// # Panics
    /// 주어진 데미지 량이 `MAX_DAMAGE`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn set_damage(&mut self, damage: u16) {
        assert!(damage <= MAX_DAMAGE);
        self.0 &= !(Self::DAMAGE_BIT_MASK << Self::DAMAGE_SHIFT);
        self.0 |= (damage & Self::DAMAGE_BIT_MASK) << Self::DAMAGE_SHIFT;
    }

    /// 데미지 량을 설정합니다.
    ///
    /// # Panics
    /// 주어진 데미지 량이 `MAX_DAMAGE`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn with_damage(mut self, damage: u16) -> Self {
        self.set_damage(damage);
        self
    }

    /// 데미지 종류를 반환합니다.
    pub fn kind(&self) -> DamageKind {
        let val = ((self.0 >> Self::KIND_SHIFT) & Self::KIND_BIT_MASK) as u8;
        // Safety: 주어지는 정수는 범위를 넘지 않습니다.
        unsafe { DamageKind::new(val).unwrap_unchecked() }
    }

    /// 데미지 종류를 설정합니다.
    pub const fn set_kind(&mut self, kind: DamageKind) {
        self.0 &= !(Self::KIND_BIT_MASK << Self::KIND_SHIFT);
        self.0 |= ((kind as u16) & Self::KIND_BIT_MASK) << Self::KIND_SHIFT;
    }

    /// 데미지 종류를 설정합니다.
    pub const fn with_kind(mut self, kind: DamageKind) -> Self {
        self.set_kind(kind);
        self
    }
}

impl BigEndian for Bitfield {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u16::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Damage {
    Miss,
    Common(u16),
    Critial(u16),
}

impl Damage {
    /// 데미지 데이터를 생성합니다.
    pub const fn miss() -> Self {
        Self::Miss
    }

    /// 데미지 데이터를 생성합니다.
    ///
    /// # Panics
    /// - 주어진 정수가 0인 경우 [`panic!`]을 호출합니다.
    /// - 주어진 정수가 `MAX_DAMAGE`보다 큰 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn common(val: u16) -> Self {
        assert!(val != 0);
        assert!(val <= MAX_DAMAGE);
        Self::Common(val)
    }

    /// 데미지 데이터를 생성합니다.
    ///
    /// # Panics
    /// - 주어진 정수가 0인 경우 [`panic!`]을 호출합니다.
    /// - 주어진 정수가 `MAX_DAMAGE`보다 큰 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn critical(val: u16) -> Self {
        assert!(val != 0);
        assert!(val <= MAX_DAMAGE);
        Self::Critial(val)
    }
}

/// 데미지 로그 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageLogData {
    /// 피격 당한 플레이어의 식별자
    pub target_id: UserId,
    /// 비트 필드 데이터
    bitfield: Bitfield,
}

impl DamageLogData {
    /// 새로운 데미지 로그 데이터를 생성합니다.
    pub const fn new(target_id: UserId, damage: Damage) -> Self {
        match damage {
            Damage::Miss => Self {
                target_id,
                bitfield: Bitfield::new().with_kind(DamageKind::Miss).with_damage(0),
            },
            Damage::Common(damage) => Self {
                target_id,
                bitfield: Bitfield::new()
                    .with_kind(DamageKind::Common)
                    .with_damage(damage),
            },
            Damage::Critial(damage) => Self {
                target_id,
                bitfield: Bitfield::new()
                    .with_kind(DamageKind::Critial)
                    .with_damage(damage),
            },
        }
    }

    /// 데미지 데이터를 반환합니다.
    pub fn as_damage(&self) -> Damage {
        match self.bitfield.kind() {
            DamageKind::Common => Damage::Common(self.bitfield.damage()),
            DamageKind::Critial => Damage::Critial(self.bitfield.damage()),
            DamageKind::Miss => Damage::Miss,
        }
    }
}

impl BigEndian for DamageLogData {
    fn byte_size() -> usize {
        UserId::byte_size() + Bitfield::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.target_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(DamageLogData)
            );
        }

        bytes
    }
}

impl TryFromBigEndian for DamageLogData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(DamageLogData)
            )
        };

        // 대상 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let target_id = UserId::from_big_endian_bytes(data);

        // 비트 필드 데이터를 가져옵니다.
        offset = offset + size;
        size = Bitfield::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = Bitfield::from_big_endian_bytes(data);

        match bitfield.kind() {
            DamageKind::Miss => {
                if bitfield.damage() != 0 {
                    return None;
                }
            }
            _ => {
                if bitfield.damage() == 0 {
                    return None;
                }
            }
        };

        Some(Self {
            target_id,
            bitfield,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_damage_common() {
        Damage::common(0);
    }

    #[test]
    #[should_panic]
    fn test_creation_damage_critical() {
        Damage::critical(0);
    }

    #[test]
    fn test_bitfield_damage_kind_miss() {
        let kind = DamageKind::Miss;
        let bitfield = Bitfield::new().with_kind(kind);
        assert_eq!(kind, bitfield.kind());
    }

    #[test]
    fn test_bitfield_damage_kind_common() {
        let kind = DamageKind::Common;
        let bitfield = Bitfield::new().with_kind(kind);
        assert_eq!(kind, bitfield.kind());
    }

    #[test]
    fn test_bitfield_damage_kind_critical() {
        let kind = DamageKind::Critial;
        let bitfield = Bitfield::new().with_kind(kind);
        assert_eq!(kind, bitfield.kind());
    }

    #[test]
    fn test_bitfield_damage() {
        let damage = 999;
        let bitfield = Bitfield::new().with_damage(damage);
        assert_eq!(damage, bitfield.damage());
    }

    #[test]
    fn test_damage_log_data() {
        let origin = DamageLogData::new(UserId::new(1341234), Damage::Critial(341));
        let bytes = origin.to_big_endian_bytes();
        let other = DamageLogData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
