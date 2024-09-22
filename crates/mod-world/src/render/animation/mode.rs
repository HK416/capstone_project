/// 애니메이션의 재생 방법입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayMode {
    /// 애니메이션을 한번만 재생합니다.
    Once, 

    /// 애니메이션을 반복해서 재생합니다.
    Loop, 
}
