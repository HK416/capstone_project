use crate::{
    components::{BigEndian, LoginToken, UserId},
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트의 커스텀 게임 대기실 팀 변경 요청 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomTeamChangeRequestPacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 로그인 토큰
    pub token: LoginToken,
    /// 대상 사용자 식별자
    pub target: UserId,
}

impl RoomTeamChangeRequestPacket {
    /// 새로운 패킷을 생성합니다.
    pub const fn new(uid: UserId, token: LoginToken, target: UserId) -> Self {
        Self { uid, token, target }
    }
}

impl Packet for RoomTeamChangeRequestPacket {
    fn packet_type() -> PacketType {
        PacketType::TeamChangeRequest
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserId::byte_size() + LoginToken::byte_size() + UserId::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());
        data.extend_from_slice(&self.target.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(RoomTeamChangeRequestPacket)
            );
        }

        RawPacket::new(Self::packet_type(), data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
                "invalid packet type. (RAW:{:?}, PACKET:{:?})",
                raw.packet_type(),
                Self::packet_type(),
            );
            return None;
        }

        // 사용자 식별자를 가져옵니다.
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

        // 대상 사용자 식별자를 가져옵니다.
        offset = offset + size;
        size = UserId::byte_size();
        data = &bytes[offset..offset + size];
        let target = UserId::from_big_endian_bytes(data);

        Some(Self { uid, token, target })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_change_request_packet() {
        let uid = UserId::new(851351);
        let token = LoginToken::new(501859034151);

        let origin = RoomTeamChangeRequestPacket::new(uid, token, uid);
        let raw = origin.as_raw();
        let other = RoomTeamChangeRequestPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
