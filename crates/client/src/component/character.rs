/// ## Tag
/// 엔터티가 캐릭터임을 식별하는 태그입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Character {
    ArisOriginal = 0,
}

impl ToString for Character {
    fn to_string(&self) -> String {
        match self {
            Character::ArisOriginal => "Aris Original",
        }
        .to_string()
    }
}

impl Into<CharacterHalo> for Character {
    fn into(self) -> CharacterHalo {
        match self {
            Character::ArisOriginal => CharacterHalo::ArisOriginalHalo,
        }
    }
}

/// ## Tag
/// 엔터티가 캐릭터의 헤일로임을 식별하는 태그입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterHalo {
    ArisOriginalHalo = 0,
}

impl ToString for CharacterHalo {
    fn to_string(&self) -> String {
        match self {
            CharacterHalo::ArisOriginalHalo => "Aris Original Halo",
        }
        .to_string()
    }
}
