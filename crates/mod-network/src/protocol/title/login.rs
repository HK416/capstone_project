use crate::{
    components::{
        BigEndian, Email, LoginFailedReason, LoginToken, Passwd, TryFromBigEndian, UserAccount,
    },
    protocol::{Packet, PacketType, RawPacket},
};

/// 클라이언트가 서버로 보내는 로그인 요청 패킷입니다.
///
/// # Note
/// 현재 이 패킷은 어떤 데이터도 담고 있지 않습니다.
///
/// # Warnings
/// 이 패킷은 암호화 후 전송되어야 합니다.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRequestPacket {
    pub email: Email,
    pub passwd: Passwd,
}

impl LoginRequestPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(email: Email, passwd: Passwd) -> Self {
        Self { email, passwd }
    }
}

impl Packet for LoginRequestPacket {
    fn packet_type() -> PacketType {
        PacketType::LoginRequest
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = Email::byte_size() + Passwd::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.email.to_big_endian_bytes());
        data.extend_from_slice(&self.passwd.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LoginRequestPacket)
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

        // 계정 이메일 정보를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = Email::byte_size();
        let mut data = &bytes[offset..offset + size];
        let email = Email::from_big_endian_bytes(data);

        // 계정 비밀번호 정보를 가져옵니다.
        offset = offset + size;
        size = Passwd::byte_size();
        data = &bytes[offset..offset + size];
        let passwd = Passwd::from_big_endian_bytes(data);

        Some(Self { email, passwd })
    }
}

/// 서버에서 클라이언트로 보내는 로그인 실패 알림 패킷입니다.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginFailedPacket {
    pub reason: LoginFailedReason,
}

impl LoginFailedPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(reason: LoginFailedReason) -> Self {
        Self { reason }
    }
}

impl Packet for LoginFailedPacket {
    fn packet_type() -> PacketType {
        PacketType::LoginFailed
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = LoginFailedReason::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.reason.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LoginFailedPacket)
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
                Self::packet_type()
            );
            return None;
        }

        // 로그인 실패 사유 정보를 가져옵니다.
        let bytes = raw.data();
        let mut offset = 0;
        let mut size = LoginFailedReason::byte_size();
        let mut data = &bytes[offset..offset + size];
        let reason = LoginFailedReason::try_from_big_endian_bytes(data)?;

        Some(Self { reason })
    }
}

/// 서버에서 클라이언트로 보내는 로그인 성공 알림 패킷입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSuccessPacket {
    pub account: UserAccount,
    pub token: LoginToken,
}

impl LoginSuccessPacket {
    /// 새로운 패킷을 생성합니다.
    pub fn new(account: UserAccount, token: LoginToken) -> Self {
        Self { account, token }
    }
}

impl Packet for LoginSuccessPacket {
    fn packet_type() -> PacketType {
        PacketType::LoginSuccess
    }

    fn as_raw(&self) -> RawPacket {
        let data_size = UserAccount::byte_size() + LoginToken::byte_size();

        // 바이트 스트림을 생성합니다.
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.account.to_big_endian_bytes());
        data.extend_from_slice(&self.token.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                data.len(),
                data_size,
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(LoginSuccessPacket)
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
        let mut size = UserAccount::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user_info = UserAccount::from_big_endian_bytes(data);

        // 로그인 토큰을 가져옵니다.
        offset = offset + size;
        size = LoginToken::byte_size();
        data = &bytes[offset..offset + size];
        let token = LoginToken::from_big_endian_bytes(data);

        Some(Self {
            account: user_info,
            token,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::{UserId, UserName};

    use super::*;

    #[test]
    fn test_login_request_packet() {
        let email = Email::default();
        let passwd = Passwd::default();

        let origin = LoginRequestPacket::new(email, passwd);
        let raw = origin.as_raw();
        let other = LoginRequestPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_login_failed_packet() {
        let reason = LoginFailedReason::Banned;

        let origin = LoginFailedPacket::new(reason);
        let raw = origin.as_raw();
        let other = LoginFailedPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_login_success_packet() {
        let user_info = UserAccount::new(UserId::new(1234566), UserName::from_str("Hello=안녕"));
        let token = LoginToken::new(123451375890);

        let origin = LoginSuccessPacket::new(user_info, token);
        let raw = origin.as_raw();
        let other = LoginSuccessPacket::from_raw(raw);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
