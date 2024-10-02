use bitflags::bitflags;



/// 사용자가 입력한 방향을 나타냅니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Direction(u8);

bitflags! {
    impl Direction : u8 {
        const Forward = 0b00000001;
        const Backward = 0b00000010;
        const Left = 0b00000100;
        const Right = 0b00001000;
        const All = 0b00001111;
    }
}

impl Direction {
    /// 사용자 입력이 정지 상태인 경우 `true`를 반환합니다.
    #[inline]
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.is_empty() 
        || *self == Direction::Forward | Direction::Backward
        || *self == Direction::Left | Direction::Right
        || *self == Direction::All 
    }
}

impl Default for Direction {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}
