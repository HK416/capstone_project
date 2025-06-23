//! 캐릭터 종류와 관련된 코드를 관리합니다.
//!

use std::fmt;

use rand::{
    distr::{Distribution, StandardUniform},
    Rng,
};

use crate::components::{BigEndian, TryFromBigEndian};

/// 캐릭터 종류의 개수입니다.
pub const NUM_CHARACTERS: usize = 4;

/// 캐릭터 모델 종류입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterKind {
    #[default]
    ArisOriginal = 0,
    MomoiOriginal = 1,
    MidoriOriginal = 2,
    YuukaOriginal = 3,
}

impl CharacterKind {
    /// 캐릭터 종류의 개수입니다.
    pub const NUM_KINDS: usize = 4;

    /// 주어진 정수로 캐릭터 모델 종류를 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(CharacterKind::ArisOriginal),
            1 => Some(CharacterKind::MomoiOriginal),
            2 => Some(CharacterKind::MidoriOriginal),
            3 => Some(CharacterKind::YuukaOriginal),
            _ => None,
        }
    }
}

impl BigEndian for CharacterKind {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        (*self as u8).to_big_endian_bytes()
    }
}

impl Distribution<CharacterKind> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CharacterKind {
        let val = rng.random_range(0..CharacterKind::NUM_KINDS as u8);
        // Safety: 주어지는 값은 범위를 벗어나지 않음
        unsafe { CharacterKind::new(val).unwrap_unchecked() }
    }
}

impl TryFromBigEndian for CharacterKind {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

impl fmt::Display for CharacterKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CharacterKind::ArisOriginal => "Aris Original",
                CharacterKind::MomoiOriginal => "Momoi Original",
                CharacterKind::MidoriOriginal => "Midori Original",
                CharacterKind::YuukaOriginal => "Yuuka Original",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_character_kind() {
        CharacterKind::new(128).unwrap();
    }

    #[test]
    fn test_creation_character_kind_aris_original() {
        let val = CharacterKind::ArisOriginal as u8;
        let kind = CharacterKind::new(val).unwrap();
        assert_eq!(CharacterKind::ArisOriginal, kind);
    }

    #[test]
    fn test_creation_character_kind_momoi_original() {
        let val = CharacterKind::MomoiOriginal as u8;
        let kind = CharacterKind::new(val).unwrap();
        assert_eq!(CharacterKind::MomoiOriginal, kind);
    }

    #[test]
    fn test_creation_character_kind_midori_original() {
        let val = CharacterKind::MidoriOriginal as u8;
        let kind = CharacterKind::new(val).unwrap();
        assert_eq!(CharacterKind::MidoriOriginal, kind);
    }

    #[test]
    fn test_creation_character_kind_yuuka_original() {
        let val = CharacterKind::YuukaOriginal as u8;
        let kind = CharacterKind::new(val).unwrap();
        assert_eq!(CharacterKind::YuukaOriginal, kind);
    }

    #[test]
    fn test_character_kind() {
        let origin = CharacterKind::MidoriOriginal;
        let bytes = origin.to_big_endian_bytes();
        let other = CharacterKind::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
