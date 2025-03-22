use std::sync::atomic::{AtomicU32, Ordering as MemOrdering};

use mod_network::components::{UserId, UserInfo, UserName};

/// 사용자 계정을 관리합니다.
///
/// 현재는 사용자 계정을 로그인 요청 순서에 따라 할당합니다.
///
pub struct AccountManager;

impl AccountManager {
    /// 사용자 계정을 할당하는 **"임시"** 함수입니다.
    pub fn alloc() -> UserInfo {
        static COUNT: AtomicU32 = AtomicU32::new(1);
        let id = COUNT.fetch_add(1, MemOrdering::AcqRel);
        let uid = UserId::new(id);
        let name = UserName::new(&format!("Player_{}", uid));
        let user_info = UserInfo::new(uid, name);
        user_info
    }
}
