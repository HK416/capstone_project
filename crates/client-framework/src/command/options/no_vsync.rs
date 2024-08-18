use std::env::Args;
use crate::app::AppBuilder;
use crate::app::AppFlags;
use crate::error::ErrorMessage;



/// 사용할 스레드 갯수를 설정하는 명령어 함수입니다.
#[inline]
pub fn callback(_: &mut Args, builder: AppBuilder) -> Result<AppBuilder, ErrorMessage> {
    Ok(builder.add_flags(AppFlags::DISABLE_VSYNC))
}
