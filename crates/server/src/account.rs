use std::sync::atomic::{AtomicU32, Ordering as MemOrdering};

use mod_network::components::{CharacterKind, GameTier, UserId, UserName};

/// 사용자 계정 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Account {
    /// 사용자 식별자입니다.
    pub uid: UserId,
    /// 사용자 이름입니다.
    pub name: UserName,
    /// 게임 티어
    pub tier: GameTier,
    /// 프로필 설정 캐릭터
    pub profile_character: Option<CharacterKind>,
}

/// 사용자 계정을 관리합니다.
///
/// 현재는 사용자 계정을 로그인 요청 순서에 따라 할당합니다.
///
pub struct AccountManager;

impl AccountManager {
    /// 사용자 계정을 할당하는 **"임시"** 함수입니다.
    pub fn alloc() -> Account {
        static COUNT: AtomicU32 = AtomicU32::new(1);
        let id = COUNT.fetch_add(1, MemOrdering::AcqRel);
        let uid = UserId::new(id);
        let name = UserName::from_str(&format!("플레이어_{}", uid));
        let tier = GameTier::Bronze;
        let profile_character = None;
        Account {
            uid,
            name,
            tier,
            profile_character,
        }
    }
}
