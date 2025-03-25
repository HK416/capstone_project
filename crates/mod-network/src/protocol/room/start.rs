use crate::{
    components::{BigEndian, StartFailedReason, TryFromBigEndian},
    protocol::{Packet, PacketType, RawPacket},
};

/// 커스텀 게임 시작에 실패했을 때 서버에서 클라이언트로 보내는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomGameStartFailedPacket {
    pub reason: StartFailedReason,
}

impl CustomGameStartFailedPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(reason: StartFailedReason) -> Self {
        Self { reason }
    }
}

impl Packet for CustomGameStartFailedPacket {
    fn packet_type() -> PacketType {
        PacketType::CustomGameStartFailed
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = StartFailedReason::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.reason.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(CustomGameStartFailedPacket)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
    }

    #[allow(unused_mut)]
    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::warn!(
                "invalid packet type. (RAW:{:?}, PACKET:{:?})",
                raw.packet_type(),
                Self::packet_type(),
            );
            return None;
        }

        // 실패 사유를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = StartFailedReason::byte_size();
        let mut data = &bytes[offset..offset + size];
        let reason = StartFailedReason::try_from_big_endian_bytes(data)?;

        Some(Self { reason })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_game_start_failed_packet() {
        let reason = StartFailedReason::PlayersNotReady;

        let origin = CustomGameStartFailedPacket::new(reason);
        let raw = origin.as_raw();
        let other = CustomGameStartFailedPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
