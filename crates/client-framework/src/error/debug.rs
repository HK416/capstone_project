use std::fmt;



/// 소스 파일, 줄, 열의 정보가 담긴 디버깅 정보입니다.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DebugInfo {
    /// 오류가 발생한 소스 파일의 이름입니다.
    pub file: &'static str, 
    /// 오류가 발생한 코드의 줄입니다.
    pub line: u32, 
    /// 오류가 발생한 코드의 열입니다.
    pub column: u32, 
}

impl fmt::Debug for DebugInfo {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FILE:{}, LINE:{}, COLUMN:{}", &self.file, &self.line, &self.column)
    }
}

impl fmt::Display for DebugInfo {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
