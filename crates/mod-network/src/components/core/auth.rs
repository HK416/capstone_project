//! 시스템 전반에서 공용으로 사용되는 인증자들을 관리합니다.
//!

use std::fmt;

use crate::components::BigEndian;

/// 게임 월드의 시대를 나타냅니다.
///
/// 클라이언트에서 항상 마지막으로 전송된 네트워크 패킷을 처리하기 위해 사용됩니다.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(u64);

impl Epoch {
    /// 게임 월드 시대의 최대 값 입니다.
    pub const MAX: Self = Self(u64::MAX);

    /// 주어진 정수로 새로운 게임 월드 시대를 생성합니다.
    pub const fn new(num: u64) -> Self {
        Self(num)
    }
}

impl BigEndian for Epoch {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u64::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for Epoch {
    fn default() -> Self {
        Self::new(0)
    }
}

impl fmt::Display for Epoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

/// 사용자의 로그인 토큰입니다.
///
/// # Warnings
/// 사용자의 클라이언트 이외의 다른 클라이언트에 해당 데이터가 노출되면 안됩니다.
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoginToken(u64);

impl LoginToken {
    /// 비어있는 로그인 토큰입니다.
    pub const NULL: Self = Self(0);

    /// 주어진 정수로 새로운 로그인 토큰을 생성합니다.
    pub const fn new(n: u64) -> Self {
        Self(n)
    }
}

impl BigEndian for LoginToken {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u64::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for LoginToken {
    fn default() -> Self {
        Self::NULL
    }
}

impl fmt::Display for LoginToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", &self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_epoch() {
        let origin = Epoch::new(1023443213523352);
        let bytes = origin.to_big_endian_bytes();
        let other = Epoch::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(Epoch::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_login_token() {
        let origin = LoginToken::new(35233522344321);
        let bytes = origin.to_big_endian_bytes();
        let other = LoginToken::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(LoginToken::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
