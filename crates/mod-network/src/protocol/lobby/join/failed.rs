//! 클라이언트가 로비 장면에 있을 때 커스텀 게임에 참여 실패 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, TryFromBigEndian},
    protocol::{Packet, PacketType, RawPacket},
};

/// 커스텀 게임 참여 실패 사유 목록의 개수입니다.
pub const NUM_JOIN_FAILED_REASONS: usize = 4;

/// 커스텀 게임 참여 실패 사유 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JoinFailedReason {
    /// 해당 커스텀 게임을 찾지 못했습니다.
    #[default]
    NotFound = 0,
    /// 커스텀 게임 수용 인원을 초과했습니다.
    FullCapacity = 1,
    /// 커스텀 게임이 이미 진행 중 입니다.
    InProgress = 2,
    /// 커스텀 게임 생성이 제한됐습니다.
    CreationLimited = 3,
    /// 커스텀 게임 관리자에 의해 차단되었습니다.
    Banned = 4,
}

impl JoinFailedReason {
    /// 주어진 정수로 `JoinFailedReason`을 생성합니다.  
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::NotFound),
            1 => Some(Self::FullCapacity),
            2 => Some(Self::InProgress),
            3 => Some(Self::CreationLimited),
            4 => Some(Self::Banned),
            _ => None,
        }
    }
}

impl BigEndian for JoinFailedReason {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let val = *self as u8;
        val.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for JoinFailedReason {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 서버가 클라이언트로 보내는 커스텀 게임 참여 실패 알림 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRoomFailedPacket {
    pub reason: JoinFailedReason,
}

impl JoinRoomFailedPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(reason: JoinFailedReason) -> Self {
        Self { reason }
    }
}

impl Packet for JoinRoomFailedPacket {
    fn packet_type() -> PacketType {
        PacketType::JoinRoomFailed
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = JoinFailedReason::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.reason.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(JoinRoomFailedPacket)
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
        let mut size = JoinFailedReason::byte_size();
        let mut data = &bytes[offset..offset + size];
        let reason = JoinFailedReason::try_from_big_endian_bytes(data)?;

        Some(Self { reason })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_join_failed_reason() {
        JoinFailedReason::new(123).unwrap();
    }

    #[test]
    fn test_join_failed_reason_not_found() {
        let val = JoinFailedReason::NotFound as u8;
        let reason = JoinFailedReason::new(val).unwrap();
        assert_eq!(JoinFailedReason::NotFound, reason);
    }

    #[test]
    fn test_join_failed_reason_fullcapacity() {
        let val = JoinFailedReason::FullCapacity as u8;
        let reason = JoinFailedReason::new(val).unwrap();
        assert_eq!(JoinFailedReason::FullCapacity, reason);
    }

    #[test]
    fn test_join_failed_reason_in_progress() {
        let val = JoinFailedReason::InProgress as u8;
        let reason = JoinFailedReason::new(val).unwrap();
        assert_eq!(JoinFailedReason::InProgress, reason);
    }

    #[test]
    fn test_join_failed_reason_creation_limited() {
        let val = JoinFailedReason::CreationLimited as u8;
        let reason = JoinFailedReason::new(val).unwrap();
        assert_eq!(JoinFailedReason::CreationLimited, reason);
    }

    #[test]
    fn test_join_failed_reason() {
        let origin = JoinFailedReason::InProgress;
        let bytes = origin.to_big_endian_bytes();
        let other = JoinFailedReason::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }

    #[test]
    fn test_join_room_failed_packet() {
        let origin = JoinRoomFailedPacket::new(JoinFailedReason::InProgress);
        let raw = origin.as_raw();
        let other = JoinRoomFailedPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
