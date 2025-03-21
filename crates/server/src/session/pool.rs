use std::{
    net::SocketAddr,
    sync::{Arc, OnceLock},
};

use mod_parallelism::collections::SkipMap;

use super::Session;

/// 현재 서버에 접속중인 세션 집합입니다.
static SESSIONS: OnceLock<SkipMap<SocketAddr, Arc<Session>>> = OnceLock::new();

/// 현재 서버에 접속중인 세션 집합을 가져옵니다.
fn get_sessions() -> &'static SkipMap<SocketAddr, Arc<Session>> {
    SESSIONS.get_or_init(|| SkipMap::default())
}

/// 현재 서버에 접속중인 세션을 관리합니다.  
/// 실제 데이터는 전역 변수에 저장되며 `SessionManager`는 전역 변수에 접근할 수 있는 인터페이스를 제공합니다.
pub struct SessionManager;

impl SessionManager {
    /// 주소에 해당하는 세션을 등록합니다.
    pub fn regist(addr: SocketAddr, session: Arc<Session>) -> Option<Arc<Session>> {
        get_sessions().insert(addr, session)
    }

    /// 주소에 해당하는 세션을 가져옵니다.
    pub fn get(addr: &SocketAddr) -> Option<Arc<Session>> {
        get_sessions().get(addr).map(|item| item.clone())
    }

    /// 주소에 해당하는 세션을 제거합니다.
    pub fn unregist(addr: &SocketAddr) -> Option<Arc<Session>> {
        get_sessions().remove(addr)
    }

    /// 현재 세션의 수를 가져옵니다.
    pub fn count() -> u32 {
        get_sessions().len() as u32
    }
}
