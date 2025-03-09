mod hierarchy;
mod motion;
mod stage;

use std::io;

use constcat::concat;

pub use self::{hierarchy::*, motion::*, stage::*};

/// 사용자 구성 파일의 상대 경로입니다.
pub const USER_CONFIG: &'static str = "user_config";
/// 폰트 에셋의 작업 디렉토리 상대 경로입니다.
pub const FONT_WORKSPACE: &'static str = "font/";
/// `NEXON Lv2 고딕` 폰트 파일의 상대 경로입니다.
pub const NEXON_LV2_GOTHIC: &'static str = concat!(FONT_WORKSPACE, "NEXON_Lv2_Gothic.ttf");
/// `NEXON Lv2 고딕` 폰트 파일의 상대 경로입니다.
pub const NEXON_LV2_GOTHIC_BOLD: &'static str =
    concat!(FONT_WORKSPACE, "NEXON_Lv2_Gothic_Bold.ttf");

/// ## Asset Load Error List
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
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
