//! 클라이언트가 로비 장면에 있을 때 랜덤매치 참여 거부 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, TryFromBigEndian},
    protocol::{Packet, PacketType, RawPacket},
};

/// 랜덤매치 참여 거부 사유 목록의 개수입니다.
pub const NUM_MATCH_REQUEST_REJECTED_REASONS: usize = 5;

/// 랜덤매치 참여 거부 사유 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MatchRequestRejectedReason {
    /// 이미 대기열에 등록되어 있습니다.
    #[default]
    AlreadyInQueue = 0,
    /// 이용이 제한된 사용자 입니다.
    Banned = 1,
    /// 게임 생성이 제한됐습니다.
    CreationLimited = 2,
}

impl MatchRequestRejectedReason {
    /// 주어진 정수로 `MatchRequestRejectedReason`을 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            2 => Some(Self::AlreadyInQueue),
            3 => Some(Self::Banned),
            4 => Some(Self::CreationLimited),
            _ => None,
        }
    }
}

impl BigEndian for MatchRequestRejectedReason {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let val = *self as u8;
        val.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for MatchRequestRejectedReason {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 서버가 클라이언트로 보내는 랜덤매치 참여 거부 알림 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchRequestRejectedPacket {
    pub reason: MatchRequestRejectedReason,
}

impl MatchRequestRejectedPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(reason: MatchRequestRejectedReason) -> Self {
        Self { reason }
    }
}

impl Packet for MatchRequestRejectedPacket {
    fn packet_type() -> PacketType {
        PacketType::MatchRequestRejected
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = MatchRequestRejectedReason::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.reason.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(MatchRequestRejectedPacket)
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
        let mut size = MatchRequestRejectedReason::byte_size();
        let mut data = &bytes[offset..offset + size];
        let reason = MatchRequestRejectedReason::try_from_big_endian_bytes(data)?;

        Some(Self { reason })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_match_request_rejected_reason() {
        MatchRequestRejectedReason::new(123).unwrap();
    }

    #[test]
    fn test_match_request_rejected_reason_already_in_queue() {
        let val = MatchRequestRejectedReason::AlreadyInQueue as u8;
        let reason = MatchRequestRejectedReason::new(val).unwrap();
        assert_eq!(MatchRequestRejectedReason::AlreadyInQueue, reason);
    }

    #[test]
    fn test_match_request_rejected_reason_banned() {
        let val = MatchRequestRejectedReason::Banned as u8;
        let reason = MatchRequestRejectedReason::new(val).unwrap();
        assert_eq!(MatchRequestRejectedReason::Banned, reason);
    }

    #[test]
    fn test_match_request_rejected_reason_creation_limited() {
        let val = MatchRequestRejectedReason::CreationLimited as u8;
        let reason = MatchRequestRejectedReason::new(val).unwrap();
        assert_eq!(MatchRequestRejectedReason::CreationLimited, reason);
    }

    #[test]
    fn test_match_request_rejected_reason() {
        let origin = MatchRequestRejectedReason::AlreadyInQueue;
        let bytes = origin.to_big_endian_bytes();
        let other = MatchRequestRejectedReason::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }

    #[test]
    fn test_match_request_rejected_packet() {
        let origin = MatchRequestRejectedPacket::new(MatchRequestRejectedReason::Banned);
        let raw = origin.as_raw();
        let other = MatchRequestRejectedPacket::from_raw(raw);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
