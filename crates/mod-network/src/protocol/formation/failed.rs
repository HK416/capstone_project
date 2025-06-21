//! 클라이언트가 캐릭터 편성 장면에 있을 때 게임 시작 실패 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, TryFromBigEndian},
    protocol::{Packet, PacketType, RawPacket},
};

/// 인게임 진입 실패 사유 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnterGameFailedResson {
    /// 한 팀이 비어있습니다.
    #[default]
    OneTeamEmpty = 0,
}

impl EnterGameFailedResson {
    /// 주어진 정수로 `EnterGameFailedResson`을 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::OneTeamEmpty),
            _ => None,
        }
    }
}

impl BigEndian for EnterGameFailedResson {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        (*self as u8).to_big_endian_bytes()
    }
}

impl TryFromBigEndian for EnterGameFailedResson {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 서버에서 클라이언트로 보내는 게임 시작 실패 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnterGameFailedPacket {
    pub reason: EnterGameFailedResson,
}

impl EnterGameFailedPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(reason: EnterGameFailedResson) -> Self {
        Self { reason }
    }
}

impl Packet for EnterGameFailedPacket {
    fn packet_type() -> PacketType {
        PacketType::EnterGameFailed
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = EnterGameFailedResson::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.reason.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(EnterGameFailedPacket)
            )
        };

        RawPacket::new(Self::packet_type(), data)
    }

    #[allow(unused_mut)]
    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
                "invalid packet type! (SRC:{:?}, DST:{:?})",
                raw.packet_type(),
                Self::packet_type(),
            );
            return None;
        }

        // 실패 사유를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = EnterGameFailedResson::byte_size();
        let mut data = &bytes[offset..offset + size];
        let reason = EnterGameFailedResson::try_from_big_endian_bytes(data)?;

        Some(Self { reason })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_enter_game_failed_reason() {
        EnterGameFailedResson::new(123).unwrap();
    }

    #[test]
    fn test_enter_game_failed_reason_one_team_empty() {
        let val = EnterGameFailedResson::OneTeamEmpty as u8;
        let reason = EnterGameFailedResson::new(val).unwrap();
        assert_eq!(EnterGameFailedResson::OneTeamEmpty, reason);
    }

    #[test]
    fn test_enter_game_failed_packet() {
        let origin = EnterGameFailedPacket::new(EnterGameFailedResson::OneTeamEmpty);
        let raw = origin.as_raw();
        let other = EnterGameFailedPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
