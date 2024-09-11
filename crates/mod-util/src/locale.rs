use serde::Deserialize;
use serde::Serialize;



/// 클라이언트 표시 언어 목록입니다.
#[repr(C)]
#[derive(Deserialize, Serialize)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppLocale {
    #[default]
    English, 
    Japanese, 
    Korean, 
}
