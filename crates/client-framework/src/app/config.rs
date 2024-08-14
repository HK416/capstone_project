use super::dpi::Dpi;
use super::locale::AppLocale;



/// 애플리케이션의 사용자 설정 데이터입니다.
#[repr(C)]
pub struct AppConfig {
    /// 애플리케이션 표시 언어입니다.
    pub locale: AppLocale,

    /// 애플리케이션 실행시 애플리케이션 창의 크기입니다.
    pub dpi: Dpi,

    /// 애플리케이션 실행시 애플리케이션 창의 전체화면 여부를 나타냅니다.
    pub fullscreen: bool,
}
