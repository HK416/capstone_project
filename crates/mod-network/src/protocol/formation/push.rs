use crate::{
    components::{BigEndian, CharacterKind, LoginToken, TryFromBigEndian, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormationSelectPacket {
    /// 사용자 계정 식별자
    pub user_id: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
    /// 선택한 캐릭터 종류
    pub character_kind: CharacterKind,
}

impl FormationSelectPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(user_id: UserId, token: LoginToken, character_kind: CharacterKind) -> Self {
        Self {
            user_id,
            token,
            character_kind,
        }
    }
}

impl Packet for FormationSelectPacket {
    fn packet_type() -> PacketType {
        PacketType::FormationSelect
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserId::byte_size() + LoginToken::byte_size() + CharacterKind::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.user_id.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.character_kind.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(FormationSelectPacket)
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
                Self::packet_type(),
            );
            return None;
        }

        // 사용자 계정 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

        // 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        // 캐릭터 종류를 가져옵니다.
        offset = offset + size;
        size = CharacterKind::byte_size();
        data = &bytes[offset..offset + size];
        let character_kind = CharacterKind::try_from_big_endian_bytes(data)?;

        Some(Self {
            user_id,
            token,
            character_kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formation_select_packet() {
        let origin = FormationSelectPacket::new(
            UserId::new(12351432),
            LoginToken::new(1513425161),
            CharacterKind::MomoiOriginal,
        );
        let raw = origin.as_raw();
        let other = FormationSelectPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
