use std::{
    error::Error,
    sync::{Arc, OnceLock},
};

use ahash::HashMap;
use parking_lot::{FairMutex, FairMutexGuard};

use crate::MaterialResource;

type PoolType = HashMap<String, Arc<MaterialResource>>;

/// 생성된 재질 쉐이더 리소스를 관리하는 풀 객체입니다.
static POOL: OnceLock<FairMutex<PoolType>> = OnceLock::new();

/// 풀 객체를 가져옵니다.
fn get_pool() -> FairMutexGuard<'static, PoolType> {
    POOL.get_or_init(|| FairMutex::new(HashMap::default()))
        .lock()
}

/// ## Material Resource Pool
/// 생성된 재질 쉐이더 리소스 객체를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `MaterialPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct MaterialPool;

impl MaterialPool {
    /// 이름에 해당하는 메쉬 객체를 가져옵니다.  
    /// 해당 메쉬 객체가 풀 객체에 존재하지 않는 경우 새로운 메쉬 객체를 생성합니다.
    pub fn get_or_init<F, E>(name: &str, func: F) -> Result<Arc<MaterialResource>, E>
    where
        F: FnOnce() -> Result<Arc<MaterialResource>, E>,
        E: Error + Send,
    {
        let mut pool = get_pool();
        match pool.get(name).cloned() {
            Some(material) => Ok(material),
            None => {
                let material = func()?;
                pool.insert(name.to_string(), material.clone());
                Ok(material)
            }
        }
    }

    /// 이름에 해당하는 메쉬 객체가 풀 객체에 존재할 경우 `true`를 반환합니다.
    pub fn contains(name: &str) -> bool {
        get_pool().contains_key(name)
    }

    /// 이름에 해당하는 메쉬 객체를 풀 객체에서 제거합니다.  
    /// 해당 메쉬 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove(name: &str) -> Option<Arc<MaterialResource>> {
        get_pool().remove(name)
    }

    /// 풀 객체에 존재하는 모든 메쉬 객체를 제거합니다.
    pub fn clear() {
        get_pool().clear()
    }
}
