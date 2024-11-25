use std::{
    hash::{Hash, Hasher},
    sync::{Arc, OnceLock},
};

use ahash::{AHasher, RandomState};
use dashmap::DashMap;

/// 생성된 텍스처 샘플러 객체를 관리하는 풀 객체입니다.
static POOL: OnceLock<DashMap<u64, Arc<wgpu::Sampler>, RandomState>> = OnceLock::new();

/// 풀 객체를 가져옵니다.
fn get_pool() -> &'static DashMap<u64, Arc<wgpu::Sampler>, RandomState> {
    POOL.get_or_init(|| DashMap::default())
}

/// [wgpu::SamplerDescriptor]의 해시 값을 가져옵니다.
fn get_hash(desc: &wgpu::SamplerDescriptor) -> u64 {
    let mut hasher = AHasher::default();
    desc.address_mode_u.hash(&mut hasher);
    desc.address_mode_v.hash(&mut hasher);
    desc.address_mode_w.hash(&mut hasher);
    desc.mag_filter.hash(&mut hasher);
    desc.min_filter.hash(&mut hasher);
    desc.mipmap_filter.hash(&mut hasher);
    desc.compare.hash(&mut hasher);
    desc.anisotropy_clamp.hash(&mut hasher);
    desc.border_color.hash(&mut hasher);
    hasher.finish()
}

/// ## Texture Sampler Pool  
/// 생성된 텍스처 샘플러 객체를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `SamplerPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct SamplerPool;

impl SamplerPool {
    /// 설명자에 해당하는 텍스처 샘플러 객체를 가져옵니다.  
    /// 해당 텍스처 샘플러 객체가 풀 객체에 존재하지 않는 경우 새로운 텍스처 샘플러 객체를 생성합니다.
    pub fn get_or_init(
        device: &wgpu::Device,
        desc: &wgpu::SamplerDescriptor,
    ) -> Arc<wgpu::Sampler> {
        get_pool()
            .entry(get_hash(desc))
            .or_insert(Arc::new(device.create_sampler(desc)))
            .clone()
    }

    /// 설명자에 해당하는 텍스처 샘플러 객체가 풀 객체에 존재할 경우 `true`를 반환합니다.
    pub fn contains(desc: &wgpu::SamplerDescriptor) -> bool {
        get_pool().contains_key(&get_hash(desc))
    }

    /// 설명자에 해당하는 텍스처 샘플러 객체를 풀 객체에서 제거합니다.  
    /// 해당 텍스처 샘플러 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove(desc: &wgpu::SamplerDescriptor) -> Option<Arc<wgpu::Sampler>> {
        get_pool()
            .remove(&get_hash(desc))
            .map(|(_, sampler)| sampler)
    }

    /// 풀 객체에 존재하는 모든 텍스처 샘플러 객체를 제거합니다.
    pub fn clear() {
        get_pool().clear()
    }
}
