/// 애플리케이션에서 사용하는 언어 목록입니다.
/// 
/// ※ 기본 값은 `English`입니다.
/// 
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppLocale {
    #[default]
    English,
    Japanese,
    Korean,
}
