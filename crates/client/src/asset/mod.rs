mod hierarchy;
mod mesh;
mod motion;
mod sound;
mod stage;
mod texture;

use std::io;

use mod_network::components::{StageLoadError, NUM_BULLETS, NUM_CHARACTERS, NUM_STAGES};

pub use self::{hierarchy::*, mesh::*, motion::*, sound::*, stage::*, texture::*};

/// 사용자 구성 파일의 상대 경로입니다.
pub const USER_CONFIG: &'static str = "user_config";

/// `NotoSans-Regular` 폰트 파일의 Uri입니다.
pub const NOTOSANS_REGULAR: &'static str = "NotoSans_Regular.ttf";
/// `NotoSans-Bold` 폰트 파일의 Uri입니다.
pub const NOTOSANS_BOLD: &'static str = "NotoSans_Bold.ttf";

/// A.R.O.N.A 캐릭터 텍스처의  `Uri`입니다.
pub const ARONA_SAD_URI: &'static str = "Arona_Sad";

/// 게임 로고 텍스처의 `Uri`입니다.
pub const GAME_LOGO_URI: &'static str = "ui/Game_Logo.png";
/// 게임 로고 텍스처의 데이터입니다.
pub const GAME_LOGO_DATA: &'static [u8; 26506] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/Game_Logo.png",
));

/// 배경화면 꾸밈 텍스처의 `Uri`입니다.
pub const BG_DECO_URI: &'static str = "BG_Deco_00";

/// 메인 로비 배경화면 텍스처의 `Uri`입니다.
pub const BG_MAIN_LOBBY_URI: &'static str = "BG_Main_Lobby";

/// 캐릭터 편성 장면 배경화면 텍스처의 `Uri`입니다.
pub const BG_FORMATION_URI: &'static str = "BG_Formation";

/// 게임 로그인 타이틀 0번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_0_URI: &'static str = "ui/BG_Login_Title_0.png";
/// 게임 로그인 타이틀 0번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_0_DATA: &'static [u8; 2744719] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_0.png"
));

/// 게임 로그인 타이틀 1번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_1_URI: &'static str = "ui/BG_Login_Title_1.png";
/// 게임 로그인 타이틀 1번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_1_DATA: &'static [u8; 3745175] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_1.png"
));
/// 게임 로그인 타이틀 2번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_2_URI: &'static str = "ui/BG_Login_Title_2.png";
/// 게임 로그인 타이틀 2번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_2_DATA: &'static [u8; 3090166] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_2.png"
));

/// 게임 로그인 타이틀 3번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_3_URI: &'static str = "ui/BG_Login_Title_3.png";
/// 게임 로그인 타이틀 3번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_3_DATA: &'static [u8; 1793237] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_3.png"
));

/// 게임 로그인 타이틀 4번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_4_URI: &'static str = "ui/BG_Login_Title_4.png";
/// 게임 로그인 타이틀 4번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_4_DATA: &'static [u8; 3338929] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_4.png"
));

/// 게임 로그인 타이틀 5번 배경화면 텍스처의 `Uri`입니다.
pub const BG_LOGIN_TITLE_5_URI: &'static str = "ui/BG_Login_Title_5.png";
/// 게임 로그인 타이틀 5번 배경화면 텍스처의 데이터입니다.
pub const BG_LOGIN_TITLE_5_DATA: &'static [u8; 3016216] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Login_Title_5.png"
));

/// 이팩트 라벨 배경 텍스처의 `Uri`입니다.
pub const BG_GROWTH_EFFECT_LABEL_URI: &'static str = "ui/BG_Growth_Effect_Label.png";
/// 이팩트 라벨 배경 텍스처의 데이터입니다.
pub const BG_GROWTH_EFFECT_LABEL_DATA: &'static [u8; 107785] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/BG_Growth_Effect_Label.png"
));

/// 나가기 아이콘 텍스처의 `Uri`입니다.
pub const HUD_EXIT_ICON_URI: &'static str = "ui/HUD_Exit_Icon.png";
/// 나가기 아이콘 텍스처의 데이터입니다.
pub const HUD_EXIT_ICON_DATA: &'static [u8; 1917] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/HUD_Exit_Icon.png"
));

/// 디테일 아이콘 텍스처의 `Uri`입니다.
pub const HUD_DETAIL_ICON_URI: &'static str = "ui/HUD_Detail_Icon.png";
/// 데테일 아이콘 텍스처의 데이터입니다.
pub const HUD_DETAIL_ICON_DATA: &'static [u8; 5310] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/HUD_Detail_Icon.png"
));

/// 취소 아이콘 텍스처의 `Uri`입니다.
pub const HUD_CANCEL_ICON_URI: &'static str = "ui/HUD_Cancel_Icon.png";
/// 취소 아이콘 텍스처의 데이터입니다.
pub const HUD_CANCEL_ICON_DATA: &'static [u8; 1436] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/HUD_Cancel_Icon.png"
));

/// 교체 아이콘 텍스처의 `Uri`입니다.
pub const HUD_CHANGE_ICON_URI: &'static str = "ui/HUD_Change_Icon.png";
/// 교체 아이콘 텍스처의 데이터입니다.
pub const HUD_CHANGE_ICON_DATA: &'static [u8; 2147] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/HUD_Change_Icon.png"
));

/// 옵션 아이콘 텍스처의 `Uri`입니다.
pub const HUD_OPTION_ICON_URI: &'static str = "ui/HUD_Option_Icon.png";
/// 옵션 아이콘 텍스처의 데이터입니다.
pub const HUD_OPTION_ICON_DATA: &'static [u8; 2181] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/ui/HUD_Option_Icon.png"
));

/// 이미지 폰트의 작업공간입니다.
pub const IMG_FONT_WORKSPACE: &'static str = "font";
/// Draw 이미지 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_DRAW: &'static str = "ImgFont_Draw";
/// Host 이미지 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_HOST_URI: &'static str = "ImgFont_Host";
/// Lose(Small) 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_LOSE_SMALL_URI: &'static str = "ImgFont_Lose_Small";
/// Lose 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_LOSE_URI: &'static str = "ImgFont_Lose";
/// Miss 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_MISS_URI: &'static str = "ImgFont_Miss";
/// Mission 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_MISSION_URI: &'static str = "ImgFont_Mission";
/// 숫자 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_NUMBER_URI: &'static str = "ImgFont_Number";
/// Ready 이미지 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_READY_URI: &'static str = "ImgFont_Ready";
/// Start 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_START_URI: &'static str = "ImgFont_Start";
/// Win(Small) 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_WIN_SMALL_URI: &'static str = "ImgFont_Win_Small";
/// Win 폰트 텍스처의 `Uri`입니다.
pub const IMG_FONT_WIN_URI: &'static str = "ImgFont_Win";

/// 스카이박스 텍스처의 작업공간입니다.
pub const BG_SKY_WORKSPACE: &'static str = "stage";
/// 스카이박스 텍스처의 `Uri`입니다.
pub const BG_SKY_URI: &'static str = "BG_Sky";

/// 캐릭터 모델의 작업 공간입니다.
pub const CHARACTER_WORKSPACES: [&'static str; NUM_CHARACTERS] = [
    "characters/aris_original",
    "characters/momoi_original",
    "characters/midori_original",
    "characters/yuuka_original",
];

/// 캐릭터 모델의 `Uri`입니다.
pub const CHARACTER_URIS: [&'static str; NUM_CHARACTERS] = [
    "aris_original",
    "momoi_original",
    "midori_original",
    "yuuka_original",
];

/// 캐릭터 이미지의 `Uri`입니다.
pub const CHARACTER_IMG_URI: &'static str = "Character_Img";
/// 캐릭터 이미지의 `Uri`입니다.
pub const CHARACTER_IMG_SMALL_URI: &'static str = "Character_Img_Small";

/// 엠블럼 배경의 `Uri`입니다.
pub const EMBLEM_BG_URI: &'static str = "Emblem_BG";

/// Ui 0번 레이아웃 이미지의 `Uri`입니다.
pub const HUD_LAYOUT_URI_00: &'static str = "HUD_Layout_00";
/// Ui 1번 레이아웃 이미지의 `Uri`입니다.
pub const HUD_LAYOUT_URI_01: &'static str = "HUD_Layout_01";
/// Ui 2번 레이아웃 이미지의 `Uri`입니다.
pub const HUD_LAYOUT_URI_02: &'static str = "HUD_Layout_02";
/// Ui 3번 레이아웃 이미지의 `Uri`입니다.
pub const HUD_LAYOUT_URI_03: &'static str = "HUD_Layout_03";

/// 아이콘의 작업공간입니다.
pub const ICON_WORKSPACE: &'static str = "ui";
/// 무기 아이콘의 `Uri`입니다.
pub const WEAPON_ICON_URI: &'static str = "Weapon_Icon";
/// 랭킹(티어) 아이콘의 `Uri`입니다.
pub const RANK_ICON_URI: &'static str = "Rank_Icons";
/// 프로필 아이콘의 `Uri`입니다.
pub const PROFILE_ICON_URI: &'static str = "Profile_Icon";
/// 인게임 schale 아이콘 텍스처의 `Uri`입니다.
pub const SCHALE_ICON_URI: &'static str = "Schale_Icon";

/// 총알 모델의 작업 공간입니다.
pub const BULLET_WORKSPACE: &'static str = "common";
/// 총알 모델의 `Uri`입니다.
pub const BULLET_URIS: [&'static str; NUM_BULLETS] = [
    "Bullet_01_Warhead",
    "Bullet_02_EnergyBoll",
    "Bullet_02_EnergyBoll_Big",
    "Bullet_03_Sphere",
];

pub const STAGE_WORKSPACES: [&'static str; NUM_STAGES] = ["stage/city"];
/// 지형 데이터의 `Uri`입니다.
pub const STAGE_URI: &'static str = "map";

/// 파티클 이펙트의 작업공간입니다.
pub const FX_WORKSPACE: &'static str = "fx";
/// 총구 화염 파티클 이펙트 텍스처의 `Uri`입니다.
pub const FX_TEX_MUZZLE_00: &'static str = "FX_TEX_Muzzle_00";
/// 총구 화염 파티클 이펙트 텍스처의 `Uri`입니다.
pub const FX_TEX_MUZZLE_01: &'static str = "FX_TEX_Muzzle_01";
/// 피격 파티클 이펙트 텍스처의 `Uri`입니다.
pub const FX_TEX_HIT_00: &'static str = "FX_TEX_Hit_00";
/// 방어막 파티클 이펙트의 `Uri`입니다.
pub const FX_MESH_SHIELD_00: &'static str = "FX_MESH_Shield_00";

/// 점령 지역의 `Uri`입니다.
pub const CAPTURE_ZONE_URI: &'static str = "Capture_Zone";

/// 배경음 사운드의 작업 공간입니다.
pub const BG_SOUND_WORKSPACE: &'static str = "sound/bg";
/// Theme_01(`Constant_Moderato`) 배경 사운드의 `Uri`입니다.
pub const BG_SOUND_THEME_01: &'static str = "Theme_01";
/// Theme_01(`Constant_Moderato`) 배경 사운드의 데이터입니다.
pub const BG_SOUND_THEME_01_DATA: &'static [u8; 1283368] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/bg/Theme_01.ogg"
));
/// Theme_03(`Mischievous_Step`) 배경 사운드의 `Uri`입니다.
pub const BG_SOUND_THEME_03: &'static str = "Theme_03";
/// Theme_14(`Step_By_Step`) 배경 사운드의 `Uri`입니다.
pub const BG_SOUND_THEME_14: &'static str = "Theme_14";
/// Theme_18(`Mechanical_JUNGLE`) 배경 사운드의 `Uri`입니다.
pub const BG_SOUND_THEME_18: &'static str = "Theme_18";
/// Theme_19(`Virtual_Storm`) 배경 사운드의 `Uri`입니다.
pub const BG_SOUND_THEME_19: &'static str = "Theme_19";
/// Theme_23(`Party_Time`) 배경 사운드의 `Uri`입니다.
pub const BG_SOUND_THEME_23: &'static str = "Theme_23";
/// Theme_31(`Hello_to_Halo`) 배경 사운드의 `Uri`입니다.
pub const BG_SOUND_THEME_31: &'static str = "Theme_31";
/// Theme_31(`Hello_to_Halo`) 배경 사운드의 데이터입니다.
pub const BG_SOUND_THEME_31_DATA: &'static [u8; 1056048] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/bg/Theme_31.ogg"
));
/// Theme_40(`Neo_City_Dive`) 배경 사운드의 `Uri`입니다.
pub const BG_SOUND_THEME_40: &'static str = "Theme_40";

/// 캐릭터 목소리 사운드의 작업 공간입니다.
pub const CV_SOUND_WORKSPACES: [&'static str; NUM_CHARACTERS] = [
    "sound/cv/aris_original",
    "sound/cv/momoi_original",
    "sound/cv/midori_original",
    "sound/cv/yuuka_original",
];
/// 캐릭터 타이틀 대사 사운드의 `Uri`입니다.
pub const CV_SOUND_TITLE: [&'static str; NUM_CHARACTERS] =
    ["Aris_Title", "Momoi_Title", "Midori_Title", "Yuuka_Title"];
/// `Aris_Original` 캐릭터 타이틀 대사 사운드의 데이터입니다.
pub const CV_ARIS_TITLE_DATA: &'static [u8; 13897] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/cv/aris_original/Aris_Title.ogg"
));
/// `Momoi_Original` 캐릭터 타이틀 대사 사운드의 데이터입니다.
pub const CV_MOMOI_TITLE_DATA: &'static [u8; 16485] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/cv/momoi_original/Momoi_Title.ogg"
));
/// `Midori_Original` 캐릭터 타이틀 대사 사운드의 데이터입니다.
pub const CV_MIDORI_TITLE_DATA: &'static [u8; 14853] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/cv/midori_original/Midori_Title.ogg"
));
/// `Yuuka_Original` 캐릭터 타이틀 대사 사운드의 데이터입니다.
pub const CV_YUUKA_TITLE_DATA: &'static [u8; 10628] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/cv/yuuka_original/Yuuka_Title.ogg"
));
/// `Yuuka_Original` 캐릭터 옵션 대사 사운드의 `Uri`입니다.
pub const CV_YUUKA_OPTION: &'static str = "Yuuka_Option";
/// `Yuuka_Original` 캐릭터 옵션 대사 사운드의 데이터입니다.
pub const CV_YUUKA_OPTION_DATA: &'static [u8; 22877] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/cv/yuuka_original/Yuuka_Option.ogg"
));
/// 인게임에서 데미지를 입을 때 발생하는 캐릭터 목소리의 `Uri`입니다.
pub const CV_BATTLE_DAMAGE: [[&'static str; 3]; NUM_CHARACTERS] = [
    [
        "Aris_Battle_Damage_1",
        "Aris_Battle_Damage_2",
        "Aris_Battle_Damage_3",
    ], // Aris_Original
    [
        "Momoi_Battle_Damage_1",
        "Momoi_Battle_Damage_2",
        "Momoi_Battle_Damage_3",
    ], // Momoi_Original
    [
        "Midori_Battle_Damage_1",
        "Midori_Battle_Damage_2",
        "Midori_Battle_Damage_3",
    ], // Midori_Original
    [
        "Yuuka_Battle_Damage_1",
        "Yuuka_Battle_Damage_2",
        "Yuuka_Battle_Damage_3",
    ], // Yuuka_Original
];
/// 인게임에서 방어막이 있을 때 데미지를 입는 경우 발생하는 캐릭터 목소리의 `Uri`입니다.
pub const CV_BATTLE_DEFENSE: [&'static str; NUM_CHARACTERS] = [
    "Aris_Battle_Defense_1",
    "Momoi_Battle_Defense_1",
    "Midori_Battle_Defense_1",
    "Yuuka_Battle_Defense_1",
];
/// 인게임에서 이동시 발생하는 캐릭터 목소리의 `Uri`입니다.
pub const CV_BATTLE_MOVE: [[&'static str; 2]; NUM_CHARACTERS] = [
    ["Aris_Battle_Move_1", "Aris_Battle_Move_2"],
    ["Momoi_Battle_Move_1", "Momoi_Battle_Move_2"],
    ["Midori_Battle_Move_1", "Midori_Battle_Move_2"],
    ["Yuuka_Battle_Move_1", "Yuuka_Battle_Move_2"],
];
/// 인게임에서 행동 불능시 발생하는 캐릭터 목소리의 `Uri`입니다.
pub const CV_BATTLE_RETIRE: [&'static str; NUM_CHARACTERS] = [
    "Aris_Battle_Retire",
    "Momoi_Battle_Retire",
    "Midori_Battle_Retire",
    "Yuuka_Battle_Retire",
];
/// 인게임에서 일반 공격시 발생하는 캐릭터 목소리의 `Uri`입니다.
pub const CV_BATTLE_SHOUT: [[&'static str; 3]; NUM_CHARACTERS] = [
    [
        "Aris_Battle_Shout_1",
        "Aris_Battle_Shout_2",
        "Aris_Battle_Shout_3",
    ],
    [
        "Momoi_Battle_Shout_1",
        "Momoi_Battle_Shout_2",
        "Momoi_Battle_Shout_3",
    ],
    [
        "Midori_Battle_Shout_1",
        "Midori_Battle_Shout_2",
        "Midori_Battle_Shout_3",
    ],
    [
        "Yuuka_Battle_Shout_1",
        "Yuuka_Battle_Shout_2",
        "Yuuka_Battle_Shout_3",
    ],
];
/// 인게임에서 스킬 발동 조건을 만족한 경우 발생하는 캐릭터 목소리의 `Uri`입니다.
pub const CV_COMMONSKILL: [&'static str; NUM_CHARACTERS] = [
    "Aris_CommonSkill",
    "Momoi_CommonSkill",
    "Midori_CommonSkill",
    "Yuuka_CommonSkill",
];
/// 인게임에서 스킬을 사용할 경우 발생하는 캐릭터 목소리의 `Uri`입니다.
pub const CV_EXSKILL_LEVEL: [[&'static str; 3]; NUM_CHARACTERS] = [
    [
        "Aris_ExSkill_Level_1",
        "Aris_ExSkill_Level_2",
        "Aris_ExSkill_Level_3",
    ],
    [
        "Momoi_ExSkill_Level_1",
        "Momoi_ExSkill_Level_2",
        "Momoi_ExSkill_Level_3",
    ],
    [
        "Midori_ExSkill_Level_1",
        "Midori_ExSkill_Level_2",
        "Midori_ExSkill_Level_3",
    ],
    [
        "Yuuka_ExSkill_Level_1",
        "Yuuka_ExSkill_Level_2",
        "Yuuka_ExSkill_Level_3",
    ],
];
/// 캐릭터 편성시 발생하는 캐릭터 목소리의 `Uri`입니다.
pub const CV_FORMATION_IN: [[&'static str; 2]; NUM_CHARACTERS] = [
    ["Aris_Formation_In_1", "Aris_Formation_In_2"],
    ["Momoi_Formation_In_1", "Momoi_Formation_In_2"],
    ["Midori_Formation_In_1", "Midori_Formation_In_2"],
    ["Yuuka_Formation_In_1", "Yuuka_Formation_In_2"],
];
/// 캐릭터 선택시 발생하는 캐릭터 목소리의 `Uri`입니다.
pub const CV_FORMATION_SELECT: [&'static str; NUM_CHARACTERS] = [
    "Aris_Formation_Select",
    "Momoi_Formation_Select",
    "Midori_Formation_Select",
    "Yuuka_Formation_Select",
];
/// 인게임 진입시 발생하는 캐릭터 목소리의 `Uri`입니다.
pub const CV_TACTIC_IN: [[&'static str; 2]; NUM_CHARACTERS] = [
    ["Aris_Tactic_In_1", "Aris_Tactic_In_2"],
    ["Momoi_Tactic_In_1", "Momoi_Tactic_In_2"],
    ["Midori_Tactic_In_1", "Midori_Tactic_In_2"],
    ["Yuuka_Tactic_In_1", "Yuuka_Tactic_In_2"],
];

/// 인게임 효과음의 작업공간입니다.
pub const SFX_WORKSPACE: &'static str = "sound/sfx";
/// 인게임에서 스킬 사용시 발생하는 효과음의 `Uri`입니다.
pub const SFX_SKILL: [&'static str; NUM_CHARACTERS] = [
    "SFX_Skill_Aris_Ex",
    "SFX_Skill_Momoi_Ex",
    "SFX_Skill_Midori_Ex",
    "SFX_Skill_Yuuka_Ex",
];
/// 인게임에서 총알 발사시 발생하는 효과음의 `Uri`입니다.
pub const SFX_COMMON: [&'static str; NUM_CHARACTERS] = [
    "SFX_Common_RG_aris_Attack",
    "SFX_Common_AR_01",
    "SFX_Common_SR_01",
    "SFX_Common_SMG_01",
];
/// 인게임에서 재장전시 발생하는 효과음의 `Uri`입니다.
pub const SFX_COMMON_RELOAD: [&'static str; NUM_CHARACTERS] = [
    "SFX_Common_RG_aris_Reload",
    "SFX_Common_AR_Reload_01",
    "SFX_Common_SR_Reload_01",
    "SFX_Common_SMG_Reload_01",
];

/// Ui 사운드의 작업 공간입니다.
pub const UI_SOUND_WORKSPACE: &'static str = "sound/ui";
/// 뒤로가기 버튼을 누를때 발생하는 Ui 사운드의 `Uri`입니다.
pub const UI_BUTTON_BACK: &'static str = "UI_Button_Back";
/// 뒤로가기 버튼을 누를때 발생하는 Ui 사운드의 데이터입니다.
pub const UI_BUTTON_BACK_DATA: &'static [u8; 4119] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/ui/UI_Button_Back.ogg"
));

/// 클릭할 때 발생하는 Ui 사운드의 `Uri`입니다.
pub const UI_BUTTON_TOUCH: &'static str = "UI_Button_Touch";
/// 클릭할 때 발생하는 Ui 사운드의 데이터입니다.
pub const UI_BUTTON_TOUCH_DATA: &'static [u8; 4021] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/ui/UI_Button_Touch.ogg"
));

/// 로딩시 발생하는 Ui 사운드의 `Uri`입니다.
pub const UI_LOADING: &'static str = "UI_Loading";
/// 로딩시 발생하는 Ui 사운드의 데이터입니다.
pub const UI_LOADING_DATA: &'static [u8; 4611] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/ui/UI_Loading.ogg"
));

/// 알림시 발생하는 Ui 사운드의 `Uri`입니다.
pub const UI_NOTICE: &'static str = "UI_Notice";
pub const UI_NOTICE_DATA: &'static [u8; 4735] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/ui/UI_Notice.ogg"
));

pub const UI_PAUSE: &'static str = "UI_Pause";
pub const UI_PAUSE_DATA: &'static [u8; 5035] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/ui/UI_Pause.ogg"
));

/// 모달 종료시 발생하는 Ui 사운드의 `Uri`입니다.
pub const UI_TURN_DOWN: &'static str = "UI_Turn_Down";
pub const UI_TURN_DOWN_DATA: &'static [u8; 9770] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/ui/UI_Turn_Down.ogg"
));

/// 모달 생성시 발생하는 Ui 사운드의 `Uri`입니다.
pub const UI_TURN_UP: &'static str = "UI_Turn_Up";
pub const UI_TURN_UP_DATA: &'static [u8; 10028] = include_bytes!(concat!(
    env!("CARGO_WORKSPACE_DIR"),
    "assets/sound/ui/UI_Turn_Up.ogg"
));

/// 게임 시작시 발생하는 UI 사운드의 `Uri`입니다.
pub const UI_START: &'static str = "UI_START_01";
/// 게임 종료시 발생하는 UI 사운드의 `Uri`입니다.
pub const UI_VICTORY_ST_01: &'static str = "UI_Victory_ST_01";

/// ## Asset Load Error List
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("invalid data")]
    InvalidData,

    /// dds 포맷의 텍스처를 읽는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to read texture for the following reason:{0}")]
    TextureError(#[from] ddsfile::Error),

    /// 에셋 파일을 구문 분석하는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to parse asset for the following reason:{0})")]
    ParsingFailed(#[from] serde_json::Error),

    /// 파일을 열거나 읽을 때 발생하는 오류입니다.
    #[error("failed to read asset for the following reason:{0})")]
    IOError(#[from] io::Error),

    #[error("failed to decode sound for the following reason:{0}")]
    DecodeFailed(#[from] rodio::decoder::DecoderError),
}

impl From<StageLoadError> for AssetError {
    fn from(value: StageLoadError) -> Self {
        match value {
            StageLoadError::InvalidData => AssetError::InvalidData,
            StageLoadError::ParsingFailed(error) => AssetError::ParsingFailed(error),
            StageLoadError::IOError(error) => AssetError::IOError(error),
        }
    }
}
