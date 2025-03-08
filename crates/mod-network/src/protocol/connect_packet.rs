use crate::components::{BigEndian, LoginToken, UserInfo};

use super::{Packet, PacketType, RawPacket};

/// 클라이언트가 서버에 연결되었을 때
/// 서버에서 클라이언트로 전송되는 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectPacket {
    pub user: UserInfo,
    pub token: LoginToken,
}

impl ConnectPacket {
    pub fn new(user: UserInfo, token: LoginToken) -> Self {
        Self { user, token }
    }
}

impl Default for ConnectPacket {
    fn default() -> Self {
        Self {
            user: UserInfo::default(),
            token: LoginToken::default(),
        }
    }
}

impl Packet for ConnectPacket {
    fn packet_type() -> PacketType {
        PacketType::Connect
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserInfo::byte_size() + LoginToken::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.user.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(ConnectPacket)
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

        // 사용자 정보를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserInfo::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user = UserInfo::from_big_endian_bytes(data);

        // 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self { user, token })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{UserId, UserName};

    use super::*;

    #[test]
    fn validation_test_packet() {
        let user = UserInfo::new(UserId::new(3141592), UserName::new("Hello안녕!"));
        let origin = ConnectPacket::new(user, LoginToken::new(123456123456123456));
        let raw_packet = origin.as_raw();
        let other = ConnectPacket::try_from_raw(raw_packet).unwrap();

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
