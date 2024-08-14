use std::env::Args;

use crate::app::AppBuilder;
use crate::app::AppFlags;
use crate::error::ErrorMessage;



/// 애플리케이션 생성 플래그 옵션을 설정하는 명령어 함수입니다.
#[inline]
pub fn show_frame_rate(_: &mut Args, builder: AppBuilder) -> Result<AppBuilder, ErrorMessage> {
    Ok(builder.add_flags(AppFlags::SHOW_FRAME_RATE))
}
