use std::fmt;

use super::BigEndian;

/// 클라이언트를 식별하기 위한 식별자입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u32);

impl ClientId {
    /// 비어있는 클라이언트 식별자입니다.
    pub const NULL: Self = Self(0);

    /// 주어진 정수로 새로운 클라이언트 식별자를 생성합니다.
    pub const fn new(num: u32) -> Self {
        Self(num)
    }
}

impl BigEndian for ClientId {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self(u32::from_big_endian_bytes(bytes))
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:o}", &self.0)
    }
}

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
impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:o}", &self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn creation_test_client_id() {
        ClientId::new(0);
    }

    #[test]
    fn validation_test_client_id() {
        let origin = ClientId::new(102344321);
        let bytes = origin.to_big_endian_bytes();
        let other = ClientId::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(ClientId::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    #[should_panic]
    fn creation_test_epoch() {
        Epoch::new(u64::MAX);
    }

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
    #[should_panic]
    fn creation_test_object_id() {
        ObjectId::new(0);
    }

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
}
