//! 게임 플레이 데이터와 관련된 코드를 관리합니다.
//!

use crate::components::BigEndian;

/// 플레이어의 게임 플레이 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GamePlayData {
    /// 상대 팀을 처치한 횟수
    pub kill_count: u16,
    /// 상대 팀에게 처치당한 횟수
    pub dead_count: u16,
}

impl BigEndian for GamePlayData {
    fn byte_size() -> usize {
        u16::byte_size() + u16::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(PlayData)
        );

        // 상대 팀을 처치한 횟수를 가져옵니다.
        let mut offset = 0;
        let mut size = u16::byte_size();
        let mut data = &bytes[offset..offset + size];
        let kill_count = u16::from_big_endian_bytes(data);

        // 상대 팀에게 처치당한 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let dead_count = u16::from_big_endian_bytes(data);

        Self {
            kill_count,
            dead_count,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.kill_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.dead_count.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PlayData)
            );
        }

        bytes
    }
}

impl Default for GamePlayData {
    fn default() -> Self {
        Self {
            kill_count: 0,
            dead_count: 0,
        }
    }
}

/// 플레이어의 게임 플레이 통계 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GamePlayStats {
    /// 상대 팀에게 입힌 총 데미지 량
    pub damage_dealt: u32,
    /// 상태 팀에게 입은 총 데미지 량
    pub damage_taken: u32,
    /// 같은 팀을 회복시킨 총 회복량
    pub healing_given: u32,
}

impl BigEndian for GamePlayStats {
    fn byte_size() -> usize {
        u32::byte_size() + u32::byte_size() + u32::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(PlayStats)
        );

        // 상대 팀에게 입힌 총 데미지 량을 가져옵니다.
        let mut offset = 0;
        let mut size = u32::byte_size();
        let mut data = &bytes[offset..offset + size];
        let damage_dealt = u32::from_big_endian_bytes(data);

        // 상대 팀에게 입은 총 데미지 량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let damage_taken = u32::from_big_endian_bytes(data);

        // 같은 팀을 회복시킨 총 회복량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let healing_given = u32::from_big_endian_bytes(data);

        Self {
            damage_dealt,
            damage_taken,
            healing_given,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.damage_dealt.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage_taken.to_big_endian_bytes());
        bytes.extend_from_slice(&self.healing_given.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(PlayStats)
            );
        }

        bytes
    }
}

impl Default for GamePlayStats {
    fn default() -> Self {
        Self {
            damage_dealt: 0,
            damage_taken: 0,
            healing_given: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_play_data() {
        let origin = GamePlayData {
            kill_count: 123,
            dead_count: 456,
        };
        let bytes = origin.to_big_endian_bytes();
        let other = GamePlayData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_play_stats() {
        let origin = GamePlayStats {
            damage_dealt: 123,
            damage_taken: 102,
            healing_given: 0,
        };
        let bytes = origin.to_big_endian_bytes();
        let other = GamePlayStats::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
