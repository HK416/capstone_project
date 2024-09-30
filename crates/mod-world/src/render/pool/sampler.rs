use std::{collections::HashMap, mem, sync::{Arc, Mutex, OnceLock}};

use lazy_static::lazy_static;

lazy_static! {
    /// 생성된 텍스처 샘플러를 관리하는 풀 객체입니다.
    static ref POOL: Mutex<HashMap<SamplerID, Arc<wgpu::Sampler>>> = Mutex::new(HashMap::with_capacity(32));
}



/// 텍스처 샘플러 풀 객체에서 텍스처 샘플러를 식별하기 위한 식별자입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SamplerID {
    address_mode_u: [u8; mem::size_of::<wgpu::AddressMode>()], 
    address_mode_v: [u8; mem::size_of::<wgpu::AddressMode>()], 
    address_mode_w: [u8; mem::size_of::<wgpu::AddressMode>()], 
    mag_filter: [u8; mem::size_of::<wgpu::FilterMode>()], 
    min_filter: [u8; mem::size_of::<wgpu::FilterMode>()], 
    mipmap_filter: [u8; mem::size_of::<wgpu::FilterMode>()], 
    load_min_clamp: [u8; mem::size_of::<f32>()], 
    load_max_clamp: [u8; mem::size_of::<f32>()], 
    compare: [u8; mem::size_of::<Option<wgpu::CompareFunction>>()], 
    anisotropy_clamp: [u8; mem::size_of::<u16>()], 
    border_color: [u8; mem::size_of::<Option<wgpu::SamplerBorderColor>>()], 
}

impl From<&wgpu::SamplerDescriptor<'_>> for SamplerID {
    #[inline]
    fn from(desc: &wgpu::SamplerDescriptor<'_>) -> Self {
        // Safe: 각 맴버의 크기는 같습니다.
        unsafe {
            Self {
                address_mode_u: mem::transmute_copy(&desc.address_mode_u), 
                address_mode_v: mem::transmute_copy(&desc.address_mode_v), 
                address_mode_w: mem::transmute_copy(&desc.address_mode_w), 
                mag_filter: mem::transmute_copy(&desc.mag_filter), 
                min_filter: mem::transmute_copy(&desc.min_filter), 
                mipmap_filter: mem::transmute_copy(&desc.mipmap_filter), 
                load_min_clamp: mem::transmute_copy(&desc.lod_min_clamp), 
                load_max_clamp: mem::transmute_copy(&desc.lod_max_clamp), 
                compare: mem::transmute_copy(&desc.compare), 
                anisotropy_clamp: mem::transmute_copy(&desc.anisotropy_clamp), 
                border_color: mem::transmute_copy(&desc.border_color), 
            }
        }
    }
}



/// 생성된 텍스처 샘플러를 관리하는 풀 객체입니다.
/// 
/// 실제 풀 객체는 전역 변수로 선언되어있으며, 
/// `SamplerPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
/// 
pub struct SamplerPool;

impl SamplerPool {
    /// 기본 선형 보간 샘플러를 반환합니다.
    #[must_use]
    pub fn linear(device: &wgpu::Device) -> Arc<wgpu::Sampler> {
        static SAMPLER: OnceLock<Arc<wgpu::Sampler>> = OnceLock::new();
        SAMPLER.get_or_init(|| {
            device.create_sampler(
                &wgpu::SamplerDescriptor {
                    label: Some("DefaultLinearSampler"), 
                    address_mode_u: wgpu::AddressMode::ClampToEdge, 
                    address_mode_v: wgpu::AddressMode::ClampToEdge, 
                    address_mode_w: wgpu::AddressMode::ClampToEdge, 
                    mag_filter: wgpu::FilterMode::Linear, 
                    min_filter: wgpu::FilterMode::Linear, 
                    mipmap_filter: wgpu::FilterMode::Linear, 
                    ..Default::default()
                }
            ).into()
        }).clone()
    }
}

impl SamplerPool {
    /// 텍스처 샘플러를 가져옵니다.
    /// 만약 텍스처 샘플러가 풀 객체에 존재하지 않을 경우 텍스처 샘플러를 생성합니다.
    #[must_use]
    pub fn get_or_init<'a>(
        device: &wgpu::Device, 
        desc: &wgpu::SamplerDescriptor<'a>
    ) -> (SamplerID, Arc<wgpu::Sampler>) {
        let sampler_id = SamplerID::from(desc);
        let mut pool_guard = POOL.lock().unwrap();
        match pool_guard.get(&sampler_id) {
            Some(sampler) => (sampler_id, sampler.clone()), 
            None => {
                let sampler = Arc::new(device.create_sampler(desc));
                pool_guard.insert(sampler_id, sampler.clone());
                (sampler_id, sampler)
            }
        }
    }

    /// 주어진 텍스처 샘플러에 해당하는 텍스처 샘플러를 풀 객체에서 가져옵니다.
    /// 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    #[must_use]
    pub fn get(sampler_id: &SamplerID) -> Option<Arc<wgpu::Sampler>> {
        let pool_guard = POOL.lock().unwrap();
        pool_guard.get(sampler_id).cloned()
    }

    /// 주어진 텍스처 샘플러 식별자에 해당하는 텍스처 샘플러가 풀 객체에 포함되어있는지 여부를 반환합니다.
    #[must_use]
    pub fn contains(sampler_id: &SamplerID) -> bool {
        let pool_guard = POOL.lock().unwrap();
        pool_guard.contains_key(sampler_id)
    }

    /// 주어진 텍스처 샘플러 식별자에 해당하는 텍스처 샘플러를 풀 객체에서 제거합니다.
    pub fn remove(sampler_id: &SamplerID) -> Option<Arc<wgpu::Sampler>> {
        let mut pool_guard = POOL.lock().unwrap();
        pool_guard.remove(sampler_id)
    }

    /// 풀 객체에 존재하는 모든 텍스처 샘플러를 제거합니다.
    pub fn clear() {
        let mut pool_guard = POOL.lock().unwrap();
        pool_guard.clear();
    }
}
