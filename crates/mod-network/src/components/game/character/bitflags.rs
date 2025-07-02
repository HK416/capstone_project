//! 캐릭터 비트 플래그 데이터와 관련된 코드를 관리합니다.
//!

/// 캐릭터의 비트 플래그 데이터입니다.
///
/// 아래 데이터가 포함됩니다.
/// - connected     | 1bit | 서버 연결 여부
/// - invincible    | 1bit | 무적 상태 여부
/// - grounded      | 1bit | 지면을 밟고 있는 여부
///
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharacterFlags(u8);

impl CharacterFlags {
    const CONNECT_BIT_MASK: u8 = 0x01;
    const CONNECT_SHIFT: usize = 0;
    const INVINCIBLE_BIT_MASK: u8 = 0x01;
    const INVINCIBLE_SHIFT: usize = 1;
    const GROUND_BIT_MASK: u8 = 0x01;
    const GROUND_SHIFT: usize = 2;

    /// 새로운 비트 필드 데이터를 생성합니다.
    pub const fn new() -> Self {
        Self(0x00)
    }

    /// 서버 연결 여부를 반환합니다.
    pub fn is_connected(&self) -> bool {
        ((self.0 >> Self::CONNECT_SHIFT) & Self::CONNECT_BIT_MASK) == Self::CONNECT_BIT_MASK
    }

    /// 서버 연결 여부를 설정합니다.
    pub const fn set_connected(&mut self, connected: bool) {
        self.0 &= !(Self::CONNECT_BIT_MASK << Self::CONNECT_SHIFT);
        self.0 |= ((connected as u8) & Self::CONNECT_BIT_MASK) << Self::CONNECT_SHIFT;
    }

    /// 서버 연결 여부를 설정합니다.
    pub const fn with_connected(mut self, connected: bool) -> Self {
        self.set_connected(connected);
        self
    }

    /// 무적 여부를 반환합니다.
    pub fn is_invincible(&self) -> bool {
        ((self.0 >> Self::INVINCIBLE_SHIFT) & Self::INVINCIBLE_BIT_MASK)
            == Self::INVINCIBLE_BIT_MASK
    }

    /// 무적 여부를 설정합니다.
    pub const fn set_invincible(&mut self, invincible: bool) {
        self.0 &= !(Self::INVINCIBLE_BIT_MASK << Self::INVINCIBLE_SHIFT);
        self.0 |= ((invincible as u8) & Self::INVINCIBLE_BIT_MASK) << Self::INVINCIBLE_SHIFT;
    }

    /// 무적 여부를 설정합니다.
    pub const fn with_invincible(mut self, invincible: bool) -> Self {
        self.set_invincible(invincible);
        self
    }

    /// 지면을 밟고 있는 여부를 반환합니다.
    pub fn is_grounded(&self) -> bool {
        ((self.0 >> Self::GROUND_SHIFT) & Self::GROUND_BIT_MASK) == Self::GROUND_BIT_MASK
    }

    /// 지면을 밟고 있는 여부를 설정합니다.
    pub const fn set_grounded(&mut self, ground: bool) {
        self.0 &= !(Self::GROUND_BIT_MASK << Self::GROUND_SHIFT);
        self.0 |= ((ground as u8) & Self::GROUND_BIT_MASK) << Self::GROUND_SHIFT;
    }

    /// 지면을 밟고 있는 여부를 설정합니다.
    pub const fn with_grounded(mut self, ground: bool) -> Self {
        self.set_grounded(ground);
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::components::CharacterFlags;

    #[test]
    fn test_character_flags_connected() {
        let flag = CharacterFlags::new().with_connected(true);
        assert_eq!(flag.is_connected(), true);

        let flag = CharacterFlags::new().with_connected(false);
        assert_eq!(flag.is_connected(), false);
    }

    #[test]
    fn test_character_flags_invincible() {
        let flag = CharacterFlags::new().with_invincible(true);
        assert_eq!(flag.is_invincible(), true);

        let flag = CharacterFlags::new().with_invincible(false);
        assert_eq!(flag.is_invincible(), false);
    }

    #[test]
    fn test_character_flags_grounded() {
        let flag = CharacterFlags::new().with_grounded(true);
        assert_eq!(flag.is_grounded(), true);

        let flag = CharacterFlags::new().with_grounded(false);
        assert_eq!(flag.is_grounded(), false);
    }
}
