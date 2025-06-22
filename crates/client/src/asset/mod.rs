mod hierarchy;
mod mesh;
mod motion;
mod stage;
mod texture;

use std::io;

use mod_network::components::{NUM_BULLETS, NUM_CHARACTERS, NUM_STAGES};

pub use self::{hierarchy::*, mesh::*, motion::*, stage::*, texture::*};

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
pub const BULLET_URIS: [&'static str; NUM_BULLETS] = ["Bullet_01_Warhead", "Bullet_02_EnergyBoll"];

pub const STAGE_WORKSPACES: [&'static str; NUM_STAGES] = ["stage/city"];

/// 지형 데이터의 `Uri`입니다.
pub const STAGE_URI: &'static str = "map";

/// 점령 지역의 `Uri`입니다.
pub const CAPTURE_ZONE_URI: &'static str = "Capture_Zone";

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
}
