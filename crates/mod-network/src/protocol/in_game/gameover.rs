use crate::{
    components::{
        BigEndian, Team, TryFromBigEndian
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는
/// 게임 종료 패킷.
#[derive(Debug, Clone, PartialEq)]
pub struct GameOverPacket {
    pub winner: Option<Team>,
    // + 플레이어별 킬 수, 총 게임 시간, 최종점수 등? 
}

impl GameOverPacket {
    pub fn new(winner: Option<Team>) -> Self {
        Self {
            winner,
        }
    }
}

impl Packet for GameOverPacket {
    fn packet_type() -> PacketType {
        PacketType::GameOver
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = Team::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        // `None`일 경우 0x2를 설정합니다.
        data.extend_from_slice(&self.winner
            .map(|team| team as u8)
            .unwrap_or(0x2)
            .to_big_endian_bytes()
        );

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(GameOverPacket)
            );
        }

        RawPacket::new(Self::packet_type(), &data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::warn!(
                "invalid packet type. (RAW:{:?}, PACKET:{:?})",
                raw.packet_type(),
                Self::packet_type()
            );
            return None;
        }

        let bytes = raw.data();
        let offset = 0;
        let size = Team::byte_size();
        let data = &bytes[offset..offset + size];

        // 점령지 데이터를 가져옵니다.
        let winner = Team::try_from_big_endian_bytes(data);

        Some(Self {
            winner,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gameover_packet() {
        let winner = Some(Team::default());

        let origin = GameOverPacket::new(winner);
        let raw = origin.as_raw();
        let other = GameOverPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
