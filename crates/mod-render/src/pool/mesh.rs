use std::sync::{Arc, OnceLock};

use ahash::RandomState;
use dashmap::DashMap;

use crate::Mesh;

/// 생성된 메쉬 객체를 관리하는 풀 객체입니다.
static POOL: OnceLock<DashMap<String, Arc<Mesh>, RandomState>> = OnceLock::new();

/// 풀 객체를 가져옵니다.
fn get_pool() -> &'static DashMap<String, Arc<Mesh>, RandomState> {
    POOL.get_or_init(|| DashMap::default())
}

/// ## Mesh Pool  
/// 생성된 메쉬 객체를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `MeshPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct MeshPool;

impl MeshPool {
    /// 이름에 해당하는 메쉬 객체를 가져옵니다.  
    /// 해당 메쉬 객체가 풀 객체에 존재하지 않는 경우 새로운 메쉬 객체를 생성합니다.
    pub fn get_or_init<S, F>(name: S, func: F) -> Arc<Mesh>
    where
        S: Into<String>,
        F: FnOnce() -> Arc<Mesh>,
    {
        get_pool().entry(name.into()).or_insert(func()).clone()
    }

    /// 이름에 해당하는 메쉬 객체가 풀 객체에 존재할 경우 `true`를 반환합니다.
    pub fn contains<S: AsRef<String>>(name: S) -> bool {
        get_pool().contains_key(name.as_ref())
    }

    /// 이름에 해당하는 메쉬 객체를 풀 객체에서 제거합니다.  
    /// 해당 메쉬 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove<S: AsRef<String>>(name: S) -> Option<(String, Arc<Mesh>)> {
        get_pool().remove(name.as_ref())
    }

    /// 풀 객체에 존재하는 모든 메쉬 객체를 제거합니다.
    pub fn clear() {
        get_pool().clear()
    }
}
