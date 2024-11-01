use std::{collections::HashMap, sync::{Arc, Mutex}};

use lazy_static::lazy_static;

use crate::render::mesh::Mesh;

lazy_static! {
    /// 생성된 메쉬 데이터를 관리하는 풀 객체입니다.
    static ref POOL: Mutex<HashMap<String, Arc<Mesh>>> = Mutex::new(HashMap::with_capacity(32));
}



/// 생성된 메쉬 데이터를 관리하는 풀 객체입니다.
/// 
/// 실제 풀 객체는 전역 변수로 선언되어 있으며, 
/// `MeshPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
/// 
pub struct MeshPool;

impl MeshPool {
    /// 주어진 이름에 해당하는 메쉬를 가져옵니다.
    /// 
    /// 만약 해당 메쉬가 풀 객체에 존재하지 않을 경우 새로운 메쉬를 생성합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn get_or_init<S, F>(name: S, func: F) -> Arc<Mesh> 
    where S: Into<String>, F: FnOnce() -> Arc<Mesh> {
        let mut lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.entry(name.into())
            .or_insert(func())
            .clone()
    }

    /// 주어진 이름에 해당하는 메쉬가 풀 객체에 포함되어있는지 여부를 반환합니다.
    #[inline]
    #[must_use]
    pub fn contains<S: AsRef<String>>(name: S) -> bool {
        let lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.contains_key(name.as_ref())
    }

    /// 주어진 이름에 해당하는 메쉬를 풀 객체에서 제거합니다.
    /// 
    /// 만약 해당 메쉬가 풀 객체에 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    /// 
    #[inline]
    pub fn remove<S: AsRef<String>>(name: S) -> Option<Arc<Mesh>> {
        let mut lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.remove(name.as_ref())
    }

    /// 풀 객체에 존재하는 모든 메쉬를 제거합니다.
    #[inline]
    pub fn clear() {
        let mut lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.clear();
    }
}
