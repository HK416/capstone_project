use bitflags::bitflags;



/// 플레이어 상태를 나타내는 플래그입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerFlags(u8);

bitflags! {
    impl PlayerFlags: u8 {
        const Forward = 0b00000001;
        const Backward = 0b00000010;
        const Left = 0b00000100;
        const Right = 0b00001000;
        const Lowest = 0b00001111;
        const Fire = 0b00010000;
    }
}

impl PlayerFlags {
    /// 사용자 입력이 정지 상태인 경우 `true`를 반환합니다.
    #[inline]
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        let flag = *self & PlayerFlags::Lowest;
        flag.is_empty()
        || flag == PlayerFlags::Forward | PlayerFlags::Backward
        || flag == PlayerFlags::Left | PlayerFlags::Right
        || flag == PlayerFlags::Lowest
    }

    /// 사용자 입력의 방향을 가져옵니다.
    #[must_use]
    pub fn get_direction(&self) -> gmm::Vector {
        let mut vector = gmm::Vector::ZERO;
        
        if self.contains(PlayerFlags::Forward) {
            vector += gmm::Vector::Z;
        }

        if self.contains(PlayerFlags::Backward) {
            vector += gmm::Vector::NEG_Z;
        }

        if self.contains(PlayerFlags::Left) {
            vector += gmm::Vector::NEG_X;
        }

        if self.contains(PlayerFlags::Right) {
            vector += gmm::Vector::X;
        }

        vector
    }
}

impl Default for PlayerFlags {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}
