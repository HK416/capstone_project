//! 게임 로그인 실패 패킷과 관련된 코드를 관리합니다.
//!

use crate::{
    components::{BigEndian, TryFromBigEndian},
    protocol::{Packet, PacketType, RawPacket},
};

/// 로그인 실패 사유 개수입니다.
pub const NUM_LOGIN_FAILED_REASONS: usize = 2;

/// 로그인 실패 사유 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoginFailedReason {
    /// 이메일 또는 비밀번호가 잘못됐습니다.
    Invalid = 0,
    /// 계정이 서버 관리자에 의해 차단당했습니다.
    Banned = 1,
}

impl LoginFailedReason {
    /// 주어진 정수로 새로운 `LoginFailedReason`을 생성합니다.  
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Invalid),
            1 => Some(Self::Banned),
            _ => None,
        }
    }
}

impl BigEndian for LoginFailedReason {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        (*self as u8).to_big_endian_bytes()
    }
}

impl TryFromBigEndian for LoginFailedReason {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 서버에서 클라이언트로 보내는 로그인 실패 패킷입니다.
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
        // 바이트 스트림을 생성합니다.
        let data_size = LoginFailedReason::byte_size();
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&self.reason.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
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
            log::error!(
                "invalid packet type. (SRC:{:?}, DST:{:?})",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_login_failed_reason() {
        LoginFailedReason::new(123).unwrap();
    }

    #[test]
    fn test_login_failed_reason_invalid() {
        let val = LoginFailedReason::Invalid as u8;
        let reason = LoginFailedReason::new(val).unwrap();
        assert_eq!(LoginFailedReason::Invalid, reason);
    }

    #[test]
    fn test_login_failed_reason_banned() {
        let val = LoginFailedReason::Banned as u8;
        let reason = LoginFailedReason::new(val).unwrap();
        assert_eq!(LoginFailedReason::Banned, reason);
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
}
