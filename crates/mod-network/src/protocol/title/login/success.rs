//! 게임 로그인 성공 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{
        BigEndian, GameTier, LoginToken, ProfileIcon, TryFromBigEndian, UserId, UserName,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 서버에서 클라이언트로 보내는 로그인 성공 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSuccessPacket {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 이름
    pub name: UserName,
    /// 게임 티어
    pub tier: GameTier,
    /// 프로필 아이콘
    pub profile_icon: ProfileIcon,
    /// 로그인 토큰
    pub token: LoginToken,
}

impl LoginSuccessPacket {
    /// 새로운 로그인 성공 패킷을 생성합니다.
    pub fn new(
        uid: UserId,
        name: UserName,
        tier: GameTier,
        profile_icon: ProfileIcon,
        token: LoginToken,
    ) -> Self {
        Self {
            uid,
            name,
            profile_icon,
            tier,
            token,
        }
    }
}

impl Packet for LoginSuccessPacket {
    fn packet_type() -> PacketType {
        PacketType::LoginSuccess
    }

    fn as_raw(&self) -> RawPacket {
        // 바이트 스트림을 생성합니다.
        let data_size = UserId::byte_size()
            + UserName::byte_size()
            + GameTier::byte_size()
            + ProfileIcon::byte_size()
            + LoginToken::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.uid.to_big_endian_bytes());
        data.extend_from_slice(&self.name.to_big_endian_bytes());
        data.extend_from_slice(&self.tier.to_big_endian_bytes());
        data.extend_from_slice(&self.profile_icon.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LoginSuccessPacket)
            )
        };

        RawPacket::new(Self::packet_type(), data)
    }

    fn try_from_raw(raw: RawPacket) -> Option<Self> {
        // 패킷 종류가 일치하는지 확인합니다.
        if raw.packet_type() != Self::packet_type() {
            log::error!(
                "invalid packet type. (SRC:{:?}, DST:{:?})",
                raw.packet_type(),
                Self::packet_type()
            );
            return None;
        }

        // 사용자 식별자를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let uid = UserId::from_big_endian_bytes(data);

        // 사용자 이름을 가져옵니다.
        offset = offset + size;
        size = UserName::byte_size();
        data = &bytes[offset..offset + size];
        let name = UserName::from_big_endian_bytes(data);

        // 게임 티어를 가져옵니다.
        offset = offset + size;
        size = GameTier::byte_size();
        data = &bytes[offset..offset + size];
        let tier = GameTier::try_from_big_endian_bytes(data)?;

        // 프로필 아이콘을 가져옵니다.
        offset = offset + size;
        size = ProfileIcon::byte_size();
        data = &bytes[offset..offset + size];
        let profile_icon = ProfileIcon::try_from_big_endian_bytes(data)?;

        // 로그인 토큰 데이터를 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self {
            uid,
            name,
            profile_icon,
            tier,
            token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_success_packet() {
        let origin = LoginSuccessPacket::new(
            UserId::new(12345),
            UserName::from_str("유우카"),
            GameTier::Silver,
            ProfileIcon::GroupMillennium,
            LoginToken::new(1351616161),
        );
        let raw = origin.as_raw();
        let other = LoginSuccessPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
