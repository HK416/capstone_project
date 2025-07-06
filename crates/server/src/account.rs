use std::sync::atomic::{AtomicU32, Ordering as MemOrdering};

use mod_network::components::{GameTier, ProfileIcon, UserId, UserName};
use crate::data::{DbConnection, UserInfo};
use futures::executor::block_on;

/// 사용자 계정 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Account {
    /// 사용자 식별자입니다.
    pub uid: UserId,
    /// 사용자 이름입니다.
    pub name: UserName,
    /// 게임 티어
    pub tier: GameTier,
    /// 프로필 아이콘
    pub profile_icon: ProfileIcon,
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
        let tier = GameTier::default();
        let profile_icon = ProfileIcon::default();
        let account = Account {
            uid,
            name,
            tier,
            profile_icon,
        };

        // DB에 정보를 저장합니다.
        let conn = DbConnection::get_connection();
        let user_info = UserInfo {
            name: account.name.to_string(),
            tier: account.tier as u8,
            profile_icon: account.profile_icon as u8,
        };

        // 비동기로 실행시키고 account 리턴(저장 완료를 기다리지 않음)
        tokio::spawn(async move {
            conn.set_user_info(&uid, &user_info).await
                .expect("Failed to set user info in database");

            // 새로 계정이 생성되는 경우에는 즉시 DB 백업
            conn.save().await
                .expect("Failed to save database");
        });

        account
    }

    pub fn load(uid: UserId) -> Option<Account> {
        // DB에서 사용자 정보를 가져옵니다.
        let conn = DbConnection::get_connection();
        
        block_on(async {
            match conn.get_user_info(&uid).await {
                Ok(Some(user_info)) => Some(Account {
                    uid,
                    name: UserName::from_str(&user_info.name),
                    tier: GameTier::new(user_info.tier)?,
                    profile_icon: ProfileIcon::new(user_info.profile_icon)?,
                }),
                Ok(None) => None,
                Err(_) => panic!("Failed to load user info from database"),
            }
        })
    }
}
