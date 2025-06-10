//! 플레이어 티어와 관련된 코드를 관리합니다.
//!

/// 게임 티어 목록입니다.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GameTier {
    #[default]
    Bronze = 0,
    Silver = 1,
    Gold = 2,
    Platinum = 3,
}

impl GameTier {
    /// 주어진 값으로 게임 티어를 생성합니다.
    ///
    /// 주어진 값이 범위를 벗어나는 경우 `None`을 반환합니다.
    ///
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(GameTier::Bronze),
            1 => Some(GameTier::Silver),
            2 => Some(GameTier::Gold),
            3 => Some(GameTier::Platinum),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_creation_game_tier() {
        GameTier::new(123).unwrap();
    }

    #[test]
    fn test_creation_game_tier_bronze() {
        let val = GameTier::Bronze as u8;
        let tier = GameTier::new(val).unwrap();
        assert_eq!(GameTier::Bronze, tier);
    }

    #[test]
    fn test_creation_game_tier_silver() {
        let val = GameTier::Silver as u8;
        let tier = GameTier::new(val).unwrap();
        assert_eq!(GameTier::Silver, tier);
    }

    #[test]
    fn test_creation_game_tier_gold() {
        let val = GameTier::Gold as u8;
        let tier = GameTier::new(val).unwrap();
        assert_eq!(GameTier::Gold, tier);
    }

    #[test]
    fn test_creation_game_tier_platinum() {
        let val = GameTier::Platinum as u8;
        let tier = GameTier::new(val).unwrap();
        assert_eq!(GameTier::Platinum, tier);
    }
}
