use crate::components::{BigEndian, TryFromBigEndian};

/// 캐릭터 선택 실패 사유 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SelectResult {
    /// 캐릭터 선택에 성공했습니다.
    Success = 0,
    /// 캐릭터가 중복됩니다.
    Duplicates = 1,
    /// 캐릭터가 금지됬습니다.
    Banned = 2,
}

impl SelectResult {
    /// 주어진 정수로 `SelectResult`를 생성합니다.  
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Success),
            1 => Some(Self::Duplicates),
            2 => Some(Self::Banned),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(SelectResult),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for SelectResult {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let val = *self as u8;
        val.to_big_endian_bytes()
    }
}

impl Default for SelectResult {
    fn default() -> Self {
        Self::Success
    }
}

impl TryFromBigEndian for SelectResult {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_create_select_result() {
        SelectResult::new(123).unwrap();
    }

    #[test]
    fn test_select_result() {
        let origin = SelectResult::Duplicates;
        let bytes = origin.to_big_endian_bytes();
        let other = SelectResult::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
