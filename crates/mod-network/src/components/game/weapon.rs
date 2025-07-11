//! 무기와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 남은 총알 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BulletData {
    /// 총알 번호
    pub sequence: u16,
    /// 공격 당 총알 발사 횟수
    pub fires_per_attack: u16,
    /// 남은 총알 수 입니다.
    pub remaining: u16,
    /// 최대 총알 수 입니다. 최대 총알 수가 0인 경우 무한대를 의미합니다.
    maximum: u16,
}

impl BulletData {
    /// 새로운 남은 총알 데이터를 생성합니다.
    ///
    /// # Panics
    /// 주어진 `remaining`이 `maximum`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(remaining: u16, maximum: u16) -> Self {
        assert!(remaining <= maximum, "invalid data!");
        Self {
            sequence: 0,
            fires_per_attack: 0,
            remaining,
            maximum,
        }
    }

    /// 새로운 남은 총알 데이터를 생성합니다.
    pub const fn splat(maximum: u16) -> Self {
        Self {
            sequence: 0,
            fires_per_attack: 0,
            remaining: maximum,
            maximum,
        }
    }

    /// 최대 총알의 수를 가져옵니다.
    pub fn num_maximum_bullets(&self) -> u16 {
        self.maximum
    }

    /// 남은 총알의 비율을 반환합니다.
    ///
    /// 남은 총알이 무한대인 경우 `None`을 반환합니다.
    ///
    pub fn percent(&self) -> Option<f32> {
        (self.maximum != 0).then(|| self.remaining as f32 / self.maximum as f32)
    }
}

impl BigEndian for BulletData {
    fn byte_size() -> usize {
        u16::byte_size() + u16::byte_size() + u16::byte_size() + u16::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.sequence.to_big_endian_bytes());
        bytes.extend_from_slice(&self.fires_per_attack.to_big_endian_bytes());
        bytes.extend_from_slice(&self.remaining.to_big_endian_bytes());
        bytes.extend_from_slice(&self.maximum.to_big_endian_bytes());

        // 바이트 배열의 길이를 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of byte array and the size of `{}` are different!",
                stringify!(BulletData)
            )
        };

        bytes
    }
}

impl Default for BulletData {
    fn default() -> Self {
        Self {
            sequence: 0,
            fires_per_attack: 0,
            remaining: 0,
            maximum: 0,
        }
    }
}

impl TryFromBigEndian for BulletData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 길이를 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of byte array and the size of `{}` are different!",
                stringify!(BulletData)
            )
        };

        // 총알 번호를 가져옵니다.
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let sequence = u16::from_big_endian_bytes(data);

        // 공격당 발사 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let fires_per_attack = u16::from_big_endian_bytes(data);

        // 남은 총알 수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let remaining = u16::from_big_endian_bytes(data);

        // 최대 총알 수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let maximum = u16::from_big_endian_bytes(data);

        (remaining <= maximum).then(|| Self {
            sequence,
            fires_per_attack,
            remaining,
            maximum,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_bullet_data() {
        BulletData::new(12, 10);
    }

    #[test]
    fn test_bullet_data() {
        let origin = BulletData::new(100, 456);
        let bytes = origin.to_big_endian_bytes();
        let other = BulletData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
