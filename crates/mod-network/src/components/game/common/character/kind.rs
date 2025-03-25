use rand::{
    distr::{Distribution, StandardUniform},
    Rng,
};

use crate::components::{BigEndian, TryFromBigEndian};

/// 캐릭터 모델 종류 수 입니다.
pub const NUM_CHARACTERS: usize = 4;

/// 캐릭터 모델 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterKind {
    ArisOriginal = 0,
    MomoiOriginal = 1,
    MidoriOriginal = 2,
    YuukaOriginal = 3,
}

impl CharacterKind {
    /// 주어진 정수로 `CharacterKind`를 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(CharacterKind::ArisOriginal),
            1 => Some(CharacterKind::MomoiOriginal),
            2 => Some(CharacterKind::MidoriOriginal),
            3 => Some(CharacterKind::YuukaOriginal),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(CharacterKind),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for CharacterKind {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for CharacterKind {
    fn default() -> Self {
        CharacterKind::ArisOriginal
    }
}

impl Distribution<CharacterKind> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CharacterKind {
        let val = rng.random_range(0..NUM_CHARACTERS) as u8;

        // Safe: 주어진 값은 범위를 벗어나지 않음
        CharacterKind::new(val).unwrap_or_default()
    }
}

impl TryFromBigEndian for CharacterKind {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

impl ToString for CharacterKind {
    fn to_string(&self) -> String {
        match self {
            CharacterKind::ArisOriginal => "Aris Original",
            CharacterKind::MomoiOriginal => "Momoi Original",
            CharacterKind::MidoriOriginal => "Midori Original",
            CharacterKind::YuukaOriginal => "Yuuka Original",
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_create_character_kind() {
        CharacterKind::new(152).unwrap();
    }

    #[test]
    fn test_character_kind() {
        let origin = CharacterKind::MomoiOriginal;
        let bytes = origin.to_big_endian_bytes();
        let other = CharacterKind::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
