use crate::components::{BigEndian, JoinFailedReason, TryFromBigEndian};

use crate::protocol::{Packet, PacketType, RawPacket};

/// (커스텀) 게임 접속 실패 패킷
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinFailedPacket {
    /// 접속 실패 이유
    pub reason: JoinFailedReason,
}

impl JoinFailedPacket {
    /// 새로운 JoinFailedPacket을 생성합니다.
    pub fn new(reason: JoinFailedReason) -> Self {
        Self { reason }
    }
}

impl Packet for JoinFailedPacket
where
    Self: Sized,
{
    fn packet_type() -> PacketType {
        PacketType::JoinFailed
    }

    fn as_raw(&self) -> RawPacket {
        let bytes = self.reason.to_big_endian_bytes();
        RawPacket::new(Self::packet_type(), &bytes)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        if raw.packet_type() != Self::packet_type() {
            return None;
        }

        let data = raw.data();
        if data.len() != 1 {
            return None;
        }

        let reason = JoinFailedReason::try_from_big_endian_bytes(data)?;
        Some(Self { reason })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_join_failed_packet() {
        let reason = JoinFailedReason::InProgress;
        let origin = JoinFailedPacket::new(reason);

        let raw_packet = origin.as_raw();
        let other = JoinFailedPacket::try_from_raw(raw_packet).unwrap();

        assert_eq!(origin, other);
    }
}
