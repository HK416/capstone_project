use std::{
    hash::{Hash, Hasher},
    sync::{Arc, OnceLock},
};

use ahash::{AHasher, HashMap};
use parking_lot::{FairMutex, FairMutexGuard};

type PoolType = HashMap<Arc<wgpu::Texture>, HashMap<u64, Arc<wgpu::TextureView>>>;

/// 생성된 텍스처 뷰 객체를 관리하는 풀 객체입니다.
static POOL: OnceLock<FairMutex<PoolType>> = OnceLock::new();

/// 풀 객체를 가져옵니다.
fn get_pool() -> FairMutexGuard<'static, PoolType> {
    POOL.get_or_init(|| FairMutex::new(HashMap::default()))
        .lock()
}

/// [wgpu::TextureViewDescriptor]의 해시 값을 가져옵니다.
fn get_hash(desc: &wgpu::TextureViewDescriptor) -> u64 {
    let mut hasher = AHasher::default();
    desc.format.hash(&mut hasher);
    desc.dimension.hash(&mut hasher);
    desc.aspect.hash(&mut hasher);
    desc.base_mip_level.hash(&mut hasher);
    desc.mip_level_count.hash(&mut hasher);
    desc.base_array_layer.hash(&mut hasher);
    desc.array_layer_count.hash(&mut hasher);
    hasher.finish()
}

/// ## Texture View Pool  
/// 생성된 텍스처 뷰 객체를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `TextureViewPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct TextureViewPool;

impl TextureViewPool {
    /// 텍스처 객체와 설명자에 해당하는 텍스처 뷰 객체를 가져옵니다.  
    /// 해당 텍스처 뷰 객체가 풀 객체에 존재하지 않는 경우 새로운 텍스처 뷰 객체를 생성합니다.
    pub fn get_or_init(
        texture: &Arc<wgpu::Texture>,
        desc: &wgpu::TextureViewDescriptor,
    ) -> Arc<wgpu::TextureView> {
        let mut pool = get_pool();
        match pool.get(texture).cloned() {
            Some(mut map) => match map.get(&get_hash(desc)).cloned() {
                Some(view) => view,
                None => {
                    let view = Arc::new(texture.create_view(desc));
                    map.insert(get_hash(desc), view.clone());
                    view
                }
            },
            None => {
                let mut map = HashMap::default();
                let view = Arc::new(texture.create_view(desc));
                map.insert(get_hash(desc), view.clone());
                pool.insert(texture.clone(), map);
                view
            }
        }
    }

    /// 텍스처 객체와 설명자에 해당하는 텍스처 뷰 객체가 풀 객체에 존재할 경우 `true`를 반환합니다.
    pub fn contains(texture: &Arc<wgpu::Texture>, desc: &wgpu::TextureViewDescriptor) -> bool {
        get_pool()
            .get(texture)
            .is_some_and(|pool| pool.contains_key(&get_hash(desc)))
    }

    /// 텍스처 객체에 해당하는 텍스처 뷰 객체들을 풀 객체에서 제거합니다.  
    /// 해당 텍스처 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove(texture: &Arc<wgpu::Texture>) -> Option<Vec<Arc<wgpu::TextureView>>> {
        get_pool()
            .remove(texture)
            .map(|pool| pool.into_values().collect())
    }

    /// 풀 객체에 존재하는 모든 텍스처 뷰 객체를 제거합니다.
    pub fn clear() {
        get_pool().clear()
    }
}
