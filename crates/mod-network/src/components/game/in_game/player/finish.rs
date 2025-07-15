//! 플레이어의 게임 결과 데이터와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, UserId};

/// 플레이어의 게임 결과 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGamePlayerResultData {
    /// 사용자 식별자
    pub uid: UserId,
    /// 상대 팀을 처치한 횟수입니다.
    pub kill_count: u16,
    /// 상대 팀에게 처치당한 횟수입니다.
    pub retreat_count: u16,
    /// 상대 팀에게 입힌 총 데미지
    pub damage_dealt: u32,
    /// 상대 팀에게 입은 총 데미지
    pub damage_taken: u32,
    /// 같은 팀을 회복시킨 회복량
    pub healing_given: u32,
    /// 서버 접속 여부
    pub is_connected: bool,
}

impl InGamePlayerResultData {
    /// 새로운 `InGamePlayerResultData`를 생성합니다.
    pub const fn new(
        uid: UserId,
        kill_count: u16,
        retreat_count: u16,
        damage_dealt: u32,
        damage_taken: u32,
        healing_given: u32,
        is_connected: bool,
    ) -> Self {
        Self {
            uid,
            kill_count,
            retreat_count,
            damage_dealt,
            damage_taken,
            healing_given,
            is_connected,
        }
    }
}

impl BigEndian for InGamePlayerResultData {
    fn byte_size() -> usize {
        UserId::byte_size()
            + u16::byte_size()
            + u16::byte_size()
            + u32::byte_size()
            + u32::byte_size()
            + u32::byte_size()
            + u8::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerResultData),
            )
        };

        // 사용자 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 상대 팀을 처치한 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let kill_count = u16::from_big_endian_bytes(data);

        // 상대 팀에게 처치당한 횟수를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let retreat_count = u16::from_big_endian_bytes(data);

        // 상대 팀에게 입힌 데미지량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let damage_dealt = u32::from_big_endian_bytes(data);

        // 상대 팀에게 입은 데미지량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let damage_taken = u32::from_big_endian_bytes(data);

        // 같은 팀을 회복시킨 회복량을 가져옵니다.
        offset = offset + size;
        size = u32::byte_size();
        data = &bytes[offset..offset + size];
        let healing_given = u32::from_big_endian_bytes(data);

        // 서버 접속 여부를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let is_connected = u8::from_big_endian_bytes(data) & 0x1 == 0x1;

        Self {
            uid,
            kill_count,
            retreat_count,
            damage_dealt,
            damage_taken,
            healing_given,
            is_connected,
        }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
        bytes.extend_from_slice(&self.kill_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.retreat_count.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage_dealt.to_big_endian_bytes());
        bytes.extend_from_slice(&self.damage_taken.to_big_endian_bytes());
        bytes.extend_from_slice(&self.healing_given.to_big_endian_bytes());
        bytes.extend_from_slice(&((self.is_connected as u8) & 0x1).to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGamePlayerResultData),
            );
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_game_player_result_data() {
        let origin = InGamePlayerResultData::new(UserId::new(851341), 31, 12, 51341, 3112, 0, true);
        let bytes = origin.to_big_endian_bytes();
        let other = InGamePlayerResultData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
