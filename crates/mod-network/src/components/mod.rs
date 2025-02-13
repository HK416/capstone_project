mod attributes;
mod bullet;
mod identifier;
mod player;
mod stage;
mod state;
pub mod map;

pub use self::{attributes::*, bullet::*, identifier::*, player::*, stage::*, state::*};

/// 자료형을 Big-endian 바이트 배열로 변환하거나, Big-endian 바이트 배열로부터 자료형을 생성하는 함수 인터페이스를 제공합니다.
pub trait BigEndian {
    /// Big-endian 바이트 배열의 크기를 반환합니다.
    fn byte_size() -> usize
    where
        Self: Sized,
    {
        core::mem::size_of::<Self>()
    }

    /// Big-endian 바이트 배열로부터 자료형을 생성합니다.
    ///
    /// # Panics
    /// 바이트 배열의 크기가 자료형의 크기와 다른 경우 [`panic!`]을 호출합니다.
    ///
    fn from_big_endian_bytes(bytes: &[u8]) -> Self
    where
        Self: Sized;

    /// 자료형을 Big-endian 바이트 배열로 변환합니다.
    fn to_big_endian_bytes(&self) -> Vec<u8>;
}

impl BigEndian for i8 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for u8 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for i16 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for u16 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for i32 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for u32 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for f32 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for i64 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for u64 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for f64 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for i128 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for u128 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for [f32; 3] {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        [
            f32::from_big_endian_bytes(&bytes[0..4]),
            f32::from_big_endian_bytes(&bytes[4..8]),
            f32::from_big_endian_bytes(&bytes[8..12]),
        ]
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12);
        bytes.extend_from_slice(&self[0].to_big_endian_bytes());
        bytes.extend_from_slice(&self[1].to_big_endian_bytes());
        bytes.extend_from_slice(&self[2].to_big_endian_bytes());
        bytes
    }
}

impl BigEndian for [f32; 4] {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        [
            f32::from_big_endian_bytes(&bytes[0..4]),
            f32::from_big_endian_bytes(&bytes[4..8]),
            f32::from_big_endian_bytes(&bytes[8..12]),
            f32::from_big_endian_bytes(&bytes[12..16]),
        ]
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&self[0].to_big_endian_bytes());
        bytes.extend_from_slice(&self[1].to_big_endian_bytes());
        bytes.extend_from_slice(&self[2].to_big_endian_bytes());
        bytes.extend_from_slice(&self[3].to_big_endian_bytes());
        bytes
    }
}

/// 자료형을 Big-endian 바이트 배열로 변환하거나, Big-endian 바이트 배열로부터 자료형을 생성하는 함수 인터페이스를 제공합니다.
pub trait TryFromBigEndian: BigEndian {
    /// Big-endian 바이트 배열로부터 자료형을 생성합니다.
    /// 자료형 생성에 실패한 경우 `None`을 반환합니다.
    ///
    /// # Panics
    /// 바이트 배열의 크기가 자료형의 크기와 다른 경우 [`panic!`]을 호출합니다.
    ///
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self>
    where
        Self: Sized;
}
