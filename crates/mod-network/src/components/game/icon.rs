//! 프로필 아이콘 또는 인게임 캐릭터 아이콘과 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, TryFromBigEndian};

/// 프로필 아이콘의 개수입니다.
pub const NUM_PROFILE_ICONS: usize = 16;

/// 프로필 아이콘 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProfileIcon {
    CharacterSensei = 0,
    CharacterAris = 1,
    CharacterMomoi = 2,
    CharacterMidori = 3,
    CharacterYuuka = 4,
    CharacterYuzu = 5,
    #[default]
    GroupSchale = 6,
    GroupAbydos = 7,
    GroupGehenna = 8,
    GroupHyakkiyako = 9,
    GroupMillennium = 10,
    GroupRedWinter = 11,
    GroupShanhaijing = 12,
    GroupSRT = 13,
    GroupTrinity = 14,
    GroupValkyrie = 15,
}

impl ProfileIcon {
    /// 주어진 정수로 프로필 아이콘을 생성합니다.
    ///
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub const fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(ProfileIcon::CharacterSensei),
            1 => Some(ProfileIcon::CharacterAris),
            2 => Some(ProfileIcon::CharacterMomoi),
            3 => Some(ProfileIcon::CharacterMidori),
            4 => Some(ProfileIcon::CharacterYuuka),
            5 => Some(ProfileIcon::CharacterYuzu),
            6 => Some(ProfileIcon::GroupSchale),
            7 => Some(ProfileIcon::GroupAbydos),
            8 => Some(ProfileIcon::GroupGehenna),
            9 => Some(ProfileIcon::GroupHyakkiyako),
            10 => Some(ProfileIcon::GroupMillennium),
            11 => Some(ProfileIcon::GroupRedWinter),
            12 => Some(ProfileIcon::GroupShanhaijing),
            13 => Some(ProfileIcon::GroupSRT),
            14 => Some(ProfileIcon::GroupTrinity),
            15 => Some(ProfileIcon::GroupValkyrie),
            _ => None,
        }
    }
}

impl BigEndian for ProfileIcon {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        (*self as u8).to_big_endian_bytes()
    }
}

impl TryFromBigEndian for ProfileIcon {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

#[allow(unused_macros)]
macro_rules! test_profile_icon {
    ($name: ident, $e: expr) => {
        #[test]
        fn $name() {
            let val = $e as u8;
            let icon = ProfileIcon::new(val).unwrap();
            assert_eq!($e, icon);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    test_profile_icon!(test_profile_icon_ch_sensei, ProfileIcon::CharacterSensei);

    test_profile_icon!(test_profile_icon_ch_aris, ProfileIcon::CharacterAris);

    test_profile_icon!(test_profile_icon_ch_momoi, ProfileIcon::CharacterMomoi);

    test_profile_icon!(test_profile_icon_ch_midori, ProfileIcon::CharacterMidori);

    test_profile_icon!(test_profile_icon_ch_yuuka, ProfileIcon::CharacterYuuka);

    test_profile_icon!(test_profile_icon_ch_yuzu, ProfileIcon::CharacterYuzu);

    test_profile_icon!(test_profile_icon_group_schale, ProfileIcon::GroupSchale);

    test_profile_icon!(test_profile_icon_group_abydos, ProfileIcon::GroupAbydos);

    test_profile_icon!(test_profile_icon_group_gehenna, ProfileIcon::GroupGehenna);

    test_profile_icon!(
        test_profile_icon_group_hyakkiyako,
        ProfileIcon::GroupHyakkiyako
    );

    test_profile_icon!(
        test_profile_icon_group_millennium,
        ProfileIcon::GroupMillennium
    );

    test_profile_icon!(
        test_profile_icon_group_red_winter,
        ProfileIcon::GroupRedWinter
    );

    test_profile_icon!(
        test_profile_icon_group_shanhaijing,
        ProfileIcon::GroupShanhaijing
    );

    test_profile_icon!(test_profile_icon_group_srt, ProfileIcon::GroupSRT);

    test_profile_icon!(test_profile_icon_group_trinity, ProfileIcon::GroupTrinity);

    test_profile_icon!(test_profile_icon_group_valkyrie, ProfileIcon::GroupValkyrie);
}
