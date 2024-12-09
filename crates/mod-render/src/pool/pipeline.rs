use std::sync::{Arc, OnceLock};

use ahash::HashMap;
use parking_lot::{FairMutex, FairMutexGuard};

type PoolType = HashMap<String, Arc<wgpu::RenderPipeline>>;

/// 생성된 렌더링 파이프라인 객체를 관리하는 풀 객체입니다.
static POOL: OnceLock<FairMutex<PoolType>> = OnceLock::new();

/// 풀 객체를 가져옵니다.
fn get_pool() -> FairMutexGuard<'static, PoolType> {
    POOL.get_or_init(|| FairMutex::new(HashMap::default()))
        .lock()
}

/// ## Graphics Pipeline Pool
/// 생성된 렌더링 파이프라인 객체를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `GraphicsPipelinePool`은 풀 객체에 접근할 수 었는 인터페이스를 제공합니다.
pub struct GraphicsPipelinePool;

impl GraphicsPipelinePool {
    /// 주어진 이름에 해당하는 파이프라인 객체를 가져옵니다.  
    /// 해당 파이프라인 객체가 풀 객체에 존재하지 않는 경우 새로운 파이프라인 객체를 생성합니다.
    pub fn get_or_init<F>(name: &str, func: F) -> Arc<wgpu::RenderPipeline>
    where
        F: FnOnce() -> Arc<wgpu::RenderPipeline>,
    {
        let mut pool = get_pool();
        match pool.get(name).cloned() {
            Some(pipeline) => pipeline,
            None => {
                let pipeline = func();
                pool.insert(name.to_string(), pipeline.clone());
                pipeline
            }
        }
    }

    /// 주어진 이름에 해당하는 파이프라인 객체가 풀 객체에 존재할 경우 `true`를 반환합니다.
    pub fn contains(name: &str) -> bool {
        get_pool().contains_key(name)
    }

    /// 주어진 이름에 해당하는 파이프라인 객체를 풀 객체에서 제거합니다.
    /// 해당 파이프라인 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove(name: &str) -> Option<Arc<wgpu::RenderPipeline>> {
        get_pool().remove(name)
    }

    /// 풀 객체에 존재하는 모든 파이프라인 객체를 제거합니다.
    pub fn clear() {
        get_pool().clear();
    }
}
