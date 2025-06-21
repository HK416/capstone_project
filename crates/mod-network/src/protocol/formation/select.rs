//! 클라이언트가 캐릭터 편성 장면에 있을 때 캐릭터 선택 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, CharacterKind, LoginToken, SelectResult, TryFromBigEndian, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트에서 서버로 보내는 캐릭터 선택 요청 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSelectRequestPacket {
    /// 사용자 계정 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
    /// 선택한 캐릭터 종류
    pub character_kind: CharacterKind,
}

impl CharacterSelectRequestPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(uid: UserId, token: LoginToken, character_kind: CharacterKind) -> Self {
        Self {
            uid,
            token,
            character_kind,
        }
    }
}

impl Packet for CharacterSelectRequestPacket {
    fn packet_type() -> PacketType {
        PacketType::CharacterSelectRequest
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = UserId::byte_size() + LoginToken::byte_size() + CharacterKind::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.character_kind.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(CharacterSelectRequestPacket)
            );
        }

        RawPacket::new(Self::packet_type(), data)
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
        let uid = UserId::from_big_endian_bytes(data);

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
            uid,
            token,
            character_kind,
        })
    }
}

/// 서버에서 클라이언트로 보내는 캐릭터 선택 응답 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSelectResponsePacket {
    pub result: SelectResult,
}

impl CharacterSelectResponsePacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(result: SelectResult) -> Self {
        Self { result }
    }
}

impl Packet for CharacterSelectResponsePacket {
    fn packet_type() -> PacketType {
        PacketType::CharacterSelectResponse
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = SelectResult::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.result.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(CharacterSelectResponsePacket)
            );
        }

        RawPacket::new(Self::packet_type(), data)
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
        let mut size = SelectResult::byte_size();
        let mut data = &bytes[offset..offset + size];
        let result = SelectResult::try_from_big_endian_bytes(data)?;

        Some(Self { result })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_select_request_packet() {
        let origin = CharacterSelectRequestPacket::new(
            UserId::new(5134134),
            LoginToken::new(95141341),
            CharacterKind::MomoiOriginal,
        );
        let raw = origin.as_raw();
        let other = CharacterSelectRequestPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_character_select_response_packet() {
        let origin = CharacterSelectResponsePacket::new(SelectResult::Duplicates);
        let raw = origin.as_raw();
        let other = CharacterSelectResponsePacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
