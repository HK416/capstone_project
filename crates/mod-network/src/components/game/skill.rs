//! 플레이어 스킬 코스트와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 스킬 코스트 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkillCostData {
    /// 남은 스킬 코스트입니다.
    remaining: u16,
    /// 최대 스킬 코스트입니다. 최대 스킬 코스트가 0인 경우 무한대를 의미합니다.
    maximum: u16,
}

impl SkillCostData {
    /// 새로운 스킬 코스트 데이터를 생성합니다.
    ///
    /// # Panics
    /// 주어진 `remaining`이 `maximum`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(remaining: u16, maximum: u16) -> Self {
        assert!(remaining <= maximum, "invalid data!");
        Self { remaining, maximum }
    }

    /// 새로운 스킬 코스트 데이터를 생성합니다.
    pub const fn splat(maximum: u16) -> Self {
        Self {
            remaining: maximum,
            maximum,
        }
    }

    /// 플레이어가 스킬 코스트를 회복할 때 호출되는 함수로
    /// 남은 스킬 코스트를 주어진 `cost`만큼 추가합니다.
    ///
    /// 이때 남은 스킬 코스트는 최대 스킬 코스트를 초과할 수 없습니다.
    ///
    pub fn on_advanced(&mut self, cost: u16) {
        self.remaining = (self.remaining + cost).min(self.maximum)
    }

    /// 플레이어가 스킬을 사용하고자 할 때 호출되는 함수로
    /// 남은 스킬 코스트가 최대 스킬 코스트와 같을 경우 `true`를 반환 후
    /// 남은 스킬 코스트를 0으로 설정합니다.
    pub fn on_fired(&mut self) -> bool {
        if self.remaining == self.maximum {
            self.remaining = 0;
            return true;
        }
        return false;
    }

    /// 남은 스킬 코스트를 가져옵니다.
    pub fn num_remaining_cost(&self) -> u16 {
        self.remaining
    }

    /// 최대 스킬 코스트를 가져옵니다.
    pub fn num_maximum_cost(&self) -> u16 {
        self.maximum
    }

    /// 남은 스킬 코스트 비율을 반환합니다.
    ///
    /// 남은 스킬 코스트가 무한대인 경우 `None`을 반환합니다.
    ///
    pub fn percent(&self) -> Option<f32> {
        (self.maximum != 0).then(|| self.remaining as f32 / self.maximum as f32)
    }
}

impl BigEndian for SkillCostData {
    fn byte_size() -> usize {
        u16::byte_size() + u16::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.remaining.to_big_endian_bytes());
        bytes.extend_from_slice(&self.maximum.to_big_endian_bytes());

        // 바이트 배열의 길이를 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of byte array and the size of `{}` are different!",
                stringify!(SkillCostData),
            )
        };

        bytes
    }
}

impl Default for SkillCostData {
    fn default() -> Self {
        Self {
            remaining: 0,
            maximum: 0,
        }
    }
}

impl TryFromBigEndian for SkillCostData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 길이를 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of byte array and the size of `{}` are different!",
                stringify!(SkillCostData),
            )
        };

        // 남은 스킬 코스트를 가져옵니다.
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let remaining = u16::from_big_endian_bytes(data);

        // 최대 스킬 코스트를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let maximum = u16::from_big_endian_bytes(data);

        (remaining <= maximum).then(|| Self { remaining, maximum })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_skill_cost_data() {
        SkillCostData::new(52, 11);
    }

    #[test]
    fn test_skill_cost_data() {
        let origin = SkillCostData::new(234, 500);
        let bytes = origin.to_big_endian_bytes();
        let other = SkillCostData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
