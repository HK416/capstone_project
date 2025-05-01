//! 게임 승리와 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VictoryType {
    /// 부전승
    DefaultWin = 0,
    /// 판정승
    JudgmentWin = 1,
}

impl VictoryType {
    /// 주어진 정수로 `VictoryType`을 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(VictoryType::DefaultWin),
            1 => Some(VictoryType::JudgmentWin),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(VictoryType),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for VictoryType {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for VictoryType {
    fn default() -> Self {
        Self::DefaultWin
    }
}

impl TryFromBigEndian for VictoryType {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(VictoryType),
        );

        Self::new(u8::from_big_endian_bytes(bytes))
    }
}
