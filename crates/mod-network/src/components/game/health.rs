//! 체력과 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 체력 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HealthData {
    /// 방어막 체력입니다.
    shield: u16,
    /// 남은 체력입니다.
    remaining: u16,
    /// 최대 체력입니다. 최대 체력이 0인 경우 무한대를 의미합니다.
    maximum: u16,
}

impl HealthData {
    /// 새로운 체력 데이터를 생성합니다.
    ///
    /// # Panics
    /// 주어진 `remaining`이 `maximum`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new(remaining: u16, maximum: u16) -> Self {
        Self::new_with_guard(0, remaining, maximum)
    }

    /// 새로운 체력 데이터를 생성합니다.
    ///
    /// # Panics
    /// 주어진 `remaining`이 `maximum`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub const fn new_with_guard(guard: u16, remaining: u16, maximum: u16) -> Self {
        assert!(remaining <= maximum, "invalid data!");
        Self {
            shield: guard,
            remaining,
            maximum,
        }
    }

    /// 새로운 체력 데이터를 생성합니다.
    pub const fn splat(maximum: u16) -> Self {
        Self::splat_with_guard(0, maximum)
    }

    /// 새로운 체력 데이터를 생성합니다.
    pub const fn splat_with_guard(guard: u16, maximum: u16) -> Self {
        Self {
            shield: guard,
            remaining: maximum,
            maximum,
        }
    }

    /// 플레이어가 리스폰 됐을 때 호출되는 함수로
    /// 체력 데이터를 초기화합니다.
    pub fn on_respawn(&mut self) {
        self.shield = 0;
        self.remaining = self.maximum;
    }

    /// 플레이어가 공격을 받았을 떄 호출되는 함수로
    /// 남은 체력을 주어진 `damage`만큼 줄입니다.
    ///
    /// 체력이 모두 소진된 경우 `true`를 반환합니다.
    ///
    pub fn on_damage(&mut self, damage: u16) -> bool {
        // 체력이 무한대인 경우 항상 `false`를 반환합니다.
        if self.maximum == 0 {
            return false;
        }

        // 1. 방어막 체력을 감소시킵니다.
        let diff = self.shield as i32 - damage as i32; // 값이 u16 범위를 넘을 것을 고려하여 i32로 타입 캐스팅
        self.shield = diff.max(0) as u16; // Downcast Safety: 값이 u16 범위를 넘지 않음

        // 2. 남은 체력을 감소시킵니다.
        let diff = self.remaining as i32 + diff.min(0); // 방어막을 감소시키고 남은 데미지는 음수 값
        if diff > 0 {
            self.remaining = diff as u16; // Downcast Safety: 값이 u16 범위를 넘지 않음
            return false;
        } else {
            self.remaining = 0;
            return true;
        }
    }

    /// 플레이어가 체력 회복을 받았을 때 호출되는 함수로
    /// 남은 체력을 주어진 `health`만큼 증가시킵니다.
    ///
    /// 이때 남은 체력은 최대 체력을 초과할 수 없습니다.
    ///
    /// 체력이 무한대이거나, 남은 체력이 없는 경우 `false`를 반환합니다.
    ///
    pub fn on_healing(&mut self, health: u16) -> bool {
        // 체력이 무한대이거나, 남은 체력이 0인 경우 항상 `false`를 반환합니다.
        if self.remaining == 0 || self.maximum == 0 {
            return false;
        }

        self.remaining = (self.remaining + health).min(self.maximum);
        return true;
    }

    /// 플레이어가 방어막 체력을 받았을 때 호출되는 함수로
    /// 방어막 체력을 주어진 `health`로 설정합니다.
    ///
    /// 체력이 무한대이거나, 남은 체력이 없는 경우 `false`를 반환합니다.
    ///
    pub fn on_shield(&mut self, health: u16) -> bool {
        if self.remaining == 0 || self.maximum == 0 {
            return false;
        }

        self.shield = health;
        return true;
    }

    /// 남은 체력을 반환합니다.
    pub fn num_remaining_health(&self) -> u16 {
        self.remaining
    }

    /// 최대 체력을 반환합니다.
    pub fn num_maximum_health(&self) -> u16 {
        self.maximum
    }

    /// 방어막 체력을 반환합니다.
    pub fn num_shield_health(&self) -> u16 {
        self.shield
    }

    /// 체력 비율과 방어막 체력의 비율을 반환합니다.
    ///
    /// 체력이 무한대인 경우 `None`을 반환합니다.
    ///  
    pub fn percent(&self) -> Option<(f32, f32)> {
        (self.maximum != 0).then(|| {
            let total = self.maximum + self.shield;
            let remaining = self.remaining as f32 / total as f32;
            let shield = self.shield as f32 / total as f32;
            (remaining, shield)
        })
    }
}

impl BigEndian for HealthData {
    fn byte_size() -> usize {
        u16::byte_size() + u16::byte_size() + u16::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.shield.to_big_endian_bytes());
        bytes.extend_from_slice(&self.remaining.to_big_endian_bytes());
        bytes.extend_from_slice(&self.maximum.to_big_endian_bytes());

        // 바이트 배열의 길이를 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of byte array and the size of `{}` are different!",
                stringify!(HealthData),
            )
        };

        bytes
    }
}

impl Default for HealthData {
    fn default() -> Self {
        Self {
            shield: 0,
            remaining: 0,
            maximum: 0,
        }
    }
}

impl TryFromBigEndian for HealthData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 길이를 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of byte array and the size of `{}` are different!",
                stringify!(HealthData),
            )
        };

        // 방어막 체력을 가져옵니다.
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let shield = u16::from_big_endian_bytes(data);

        // 남은 체력을 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let remaining = u16::from_big_endian_bytes(data);

        // 최대 체력을 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let maximum = u16::from_big_endian_bytes(data);

        (remaining <= maximum).then(|| Self {
            shield,
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
    fn test_creation_health_data() {
        HealthData::new(52, 14);
    }

    #[test]
    fn test_health_data() {
        let origin = HealthData::new_with_guard(12, 34, 52);
        let bytes = origin.to_big_endian_bytes();
        let other = HealthData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
