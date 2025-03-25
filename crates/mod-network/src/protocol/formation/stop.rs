use crate::{
    components::{BigEndian, GamePlayStopReason, TryFromBigEndian},
    protocol::{Packet, PacketType, RawPacket},
};

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GamePlayStopPacket {
    pub reason: GamePlayStopReason,
}

impl GamePlayStopPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(reason: GamePlayStopReason) -> Self {
        Self { reason }
    }
}

impl Packet for GamePlayStopPacket {
    fn packet_type() -> PacketType {
        PacketType::GamePlayStop
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = GamePlayStopReason::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.reason.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(FormationSelectResponsePacket)
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

        // 선택 결과를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = GamePlayStopReason::byte_size();
        let mut data = &bytes[offset..offset + size];
        let reason = GamePlayStopReason::try_from_big_endian_bytes(data)?;

        Some(Self { reason })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_play_stop_packet() {
        let origin = GamePlayStopPacket::new(GamePlayStopReason::OneTeamEmpty);
        let raw = origin.as_raw();
        let other = GamePlayStopPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
