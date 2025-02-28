use crate::components::{BigEndian, CharacterKind, LoginToken, TryFromBigEndian};

use super::{Packet, PacketType, RawPacket};

/// 클라이언트가 게임에 참가하길 희망할 때
/// 클라이언트에서 서버로 전송되는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnterStagePacket {
    pub token: LoginToken,
    pub character_kind: CharacterKind,
}

impl EnterStagePacket {
    pub fn new(token: LoginToken, character_kind: CharacterKind) -> Self {
        Self {
            token,
            character_kind,
        }
    }
}

impl Default for EnterStagePacket {
    fn default() -> Self {
        Self {
            token: LoginToken::default(),
            character_kind: CharacterKind::default(),
        }
    }
}

impl Packet for EnterStagePacket {
    fn packet_type() -> PacketType {
        PacketType::EnterStage
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = LoginToken::byte_size() + CharacterKind::byte_size();

        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.character_kind.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(EnterStagePacket)
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

        // 로그인 토큰을 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = LoginToken::byte_size();
        let mut data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        // 캐릭터 종류를 가져옵니다.
        offset = offset + size;
        size = CharacterKind::byte_size();
        data = &bytes[offset..offset + size];
        let character_kind = CharacterKind::try_from_big_endian_bytes(data)?;

        Some(Self {
            token,
            character_kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_packet() {
        let origin = EnterStagePacket::new(
            LoginToken::new(123456123456123456),
            CharacterKind::MomoiOriginal,
        );
        let raw_packet = origin.as_raw();
        let other = EnterStagePacket::try_from_raw(raw_packet).unwrap();

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
