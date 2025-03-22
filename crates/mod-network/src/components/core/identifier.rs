//! 시스템 전반에서 공용으로 사용되는 식별자들을 관리합니다.
//!

use std::fmt;

use crate::components::BigEndian;

/// 게임 월드 내 오브젝트를 식별하기 위한 식별자입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(u32);

impl ObjectId {
    /// 비어있는 오브젝트 식별자입니다.
    pub const NULL: Self = Self(0);

    /// 주어진 정수로 새로운 오브젝트 식별자를 생성합니다.
    pub const fn new(num: u32) -> Self {
        Self(num)
    }
}

impl BigEndian for ObjectId {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::NULL
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

/// 사용자를 식별하기 위한 식별자입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(u32);

impl UserId {
    /// 비어있는 사용자 식별자입니다.
    pub const NULL: Self = Self(0);

    /// 주어진 정수로 새로운 사용자 식별자를 생성합니다.
    pub const fn new(n: u32) -> Self {
        Self(n)
    }
}

impl BigEndian for UserId {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::NULL
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

/// 게임 월드를 식별하기 위한 식별자입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldId(u32);

impl WorldId {
    /// 지정되지 않은 게임 월드 식별자입니다.
    pub const NULL: Self = Self(0);

    /// 주어진 정수로 새로운 게임 월드 식별자를 생성합니다.
    pub const fn new(n: u32) -> Self {
        Self(n)
    }
}

impl BigEndian for WorldId {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl fmt::Display for WorldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_object_id() {
        let origin = ObjectId::new(3523352);
        let bytes = origin.to_big_endian_bytes();
        let other = ObjectId::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(ObjectId::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_user_id() {
        let origin = UserId::new(3523352);
        let bytes = origin.to_big_endian_bytes();
        let other = UserId::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(UserId::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_world_id() {
        let origin = WorldId::new(12345);
        let bytes = origin.to_big_endian_bytes();
        let other = WorldId::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(std::mem::size_of::<WorldId>(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
