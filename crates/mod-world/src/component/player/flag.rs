use bitflags::bitflags;



/// 플레이어 상태를 나타내는 플래그입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerFlags(u8);

bitflags! {
    impl PlayerFlags: u8 {
        const Fire = 0b00000001;
    }
}

impl Default for PlayerFlags {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}
