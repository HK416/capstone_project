use std::fmt;

use super::{BigEndian, TryFromBigEndian};

/// 클라이언트를 식별하기 위한 식별자입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(u64);

impl ClientId {
    /// 비어있는 클라이언트 식별자입니다.
    pub const NULL: Self = Self(0);

    /// 주어진 정수로 새로운 클라이언트 식별자를 생성합니다.
    ///
    /// # Panics
    /// 주어진 정수가 `0`인 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(num: u64) -> Self {
        assert_ne!(num, 0, "invalid client id");
        unsafe { Self::new_unchecked(num) }
    }

    /// 주어진 정수로 새로운 클라이언트 식별자를 생성합니다.
    pub const unsafe fn new_unchecked(num: u64) -> Self {
        Self(num)
    }
}

impl BigEndian for ClientId {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid client id")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for ClientId {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let num = u64::from_big_endian_bytes(bytes);
        if num != 0 {
            unsafe { Some(Self::new_unchecked(num)) }
        } else {
            log::error!(
                "invalid value for `{}`, (VALUE:{})",
                stringify!(ClientId),
                num
            );
            None
        }
    }
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", &self.0)
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
    ///
    /// # Panics
    /// 주어진 정수가 `u64::MAX`인 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(num: u64) -> Self {
        assert!(num != u64::MAX, "out of bounds");
        unsafe { Self::new_unchecked(num) }
    }

    /// 주어진 정수로 새로운 클라이언트 식별자를 생성합니다.
    pub const unsafe fn new_unchecked(num: u64) -> Self {
        Self(num)
    }
}

impl BigEndian for Epoch {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for Epoch {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let num = u64::from_big_endian_bytes(bytes);
        if num != u64::MAX {
            unsafe { Some(Self::new_unchecked(num)) }
        } else {
            log::error!("invalid value for `{}`, (VALUE:{})", stringify!(Epoch), num);
            None
        }
    }
}

/// 게임 월드 내 오브젝트를 식별하기 위한 식별자입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(u64);

impl ObjectId {
    /// 비어있는 오브젝트 식별자입니다.
    pub const NULL: Self = Self(0);

    /// 주어진 정수로 새로운 오브젝트 식별자를 생성합니다.
    ///
    /// # Panics
    /// 주어진 정수가 `0`인 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(num: u64) -> Self {
        assert_ne!(num, 0, "invalid object id");
        unsafe { Self::new_unchecked(num) }
    }

    /// 주어진 정수로 새로운 오브젝트 식별자를 생성합니다.
    pub const unsafe fn new_unchecked(num: u64) -> Self {
        Self(num)
    }
}

impl BigEndian for ObjectId {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid object id")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for ObjectId {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let num = u64::from_big_endian_bytes(bytes);
        if num != 0 {
            unsafe { Some(Self::new_unchecked(num)) }
        } else {
            log::error!(
                "invalid value for `{}`, (VALUE:{})",
                stringify!(ObjectId),
                num
            );
            None
        }
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", &self.0)
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
        let origin = ClientId::new(1023443213523352);
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
        let origin = ObjectId::new(1023443213523352);
        let bytes = origin.to_big_endian_bytes();
        let other = ObjectId::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(ObjectId::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
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
    ///
    /// # Panics
    /// 주어진 정수가 `0`인 경우 [`panic!`]을 호출합니다.
    ///
    pub fn new(n: u32) -> Self {
        assert_ne!(n, 0, "invalid world id");
        unsafe { Self::new_unchecked(n) }
    }

    /// 주어진 정수로 새로운 게임 월드 식별자를 생성합니다.
    pub const unsafe fn new_unchecked(n: u32) -> Self {
        Self(n)
    }
}

impl BigEndian for WorldId {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid world id")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.0.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for WorldId {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let num = u32::from_big_endian_bytes(bytes);
        if num != 0 {
            unsafe { Some(Self::new_unchecked(num)) }
        } else {
            log::error!(
                "invalid value for `{}`, (VALUE:{})",
                stringify!(WorldId),
                num
            );
            None
        }
    }
}

impl fmt::Display for WorldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", &self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn creation_test_world_id() {
        WorldId::new(0);
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
