/// 게임 월드 실행 상태를 나타냅니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GameWorldStatus {
    /// 비활성화 상태
    Closed = 0,
    /// 활성화 상태이며, 게임 참가를 허용함.
    Open = 1,
    /// 활성화 상태이며, 게임 참가를 허용하지 않음
    Running = 2,
}

impl GameWorldStatus {
    /// 주어진 정수로부터 `GameWorldStatus`를 생성합니다.  
    /// 주어진 정수가 범위를 벗어나는 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Closed),
            1 => Some(Self::Open),
            2 => Some(Self::Running),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(GameWorldStatus),
                    val
                );
                None
            }
        }
    }
}
