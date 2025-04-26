use crate::components::{BigEndian, TryFromBigEndian};

/// 플레이어 체력 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HealthPoint {
    /// 현재 플레이어의 체력입니다.
    pub current: u16,
    /// 최대 플레이어 체력입니다.  
    /// 최대 플레이어 체력이 0인 경우 무한대임을 의미합니다.
    pub maximum: u16,
}

impl HealthPoint {
    /// 새로운 플레이어 체력 데이터를 생성합니다.
    ///
    /// # Panics
    /// 주어진 `current`가 `maximum`보다 클 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(current: u16, maximum: u16) -> Self {
        assert!(current <= maximum, "health point out of range!");
        unsafe { Self::new_unchecked(current, maximum) }
    }

    /// 새로운 플레이어 체력 데이터를 생성합니다.
    ///
    /// # Safety
    /// 주어진 `current`가 `maximum`보다 클 경우 정의되지 않은 동작을 수행할 수 있습니다.
    ///
    pub unsafe fn new_unchecked(current: u16, maximum: u16) -> Self {
        Self { current, maximum }
    }

    /// 새로운 플레이어 체력 데이터를 생성합니다.
    pub fn splat(maximum: u16) -> Self {
        Self {
            current: maximum,
            maximum,
        }
    }

    /// 플레이어 체력 비율을 0부터 1사이의 값으로 반환합니다.  
    /// 플레이어 현재 체력이 최대 체력보다 클 경우 1이상의 값을 반환합니다.  
    /// 플레이어 최대 체력이 0인 경우 [`f32::INFINITY`]를 반환합니다.
    pub fn normalize(&self) -> f32 {
        if self.maximum == 0 {
            f32::INFINITY
        } else {
            self.current as f32 / self.maximum as f32
        }
    }
}

impl BigEndian for HealthPoint {
    fn byte_size() -> usize {
        u16::byte_size() + u16::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.current.to_big_endian_bytes());
        bytes.extend_from_slice(&self.maximum.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(HealthPoint)
            );
        }

        bytes
    }
}

impl Default for HealthPoint {
    fn default() -> Self {
        Self {
            current: 0,
            maximum: 0,
        }
    }
}

impl TryFromBigEndian for HealthPoint {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(HealthPoint)
        );

        // 현재 체력을 가져옵니다.
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let current = u16::from_big_endian_bytes(data);

        // 최대 체력을 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let maximum = u16::from_big_endian_bytes(data);

        if current <= maximum {
            Some(Self { current, maximum })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_health_point() {
        HealthPoint::new(1, 0);
    }

    #[test]
    fn test_health_point() {
        let origin = HealthPoint::new(123, 456);
        let bytes = origin.to_big_endian_bytes();
        let other = HealthPoint::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
