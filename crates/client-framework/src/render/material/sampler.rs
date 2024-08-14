use std::mem;
use std::sync::Arc;
use std::sync::Mutex;
use hashbrown::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    /// 생성된 텍스처 샘플러를 관리하는 풀 객체입니다.
    static ref POOL: Mutex<HashMap<SamplerID, Arc<wgpu::Sampler>>> = Mutex::new(HashMap::with_capacity(8));
}



/// 텍스처 샘플러의 식별자입니다.
/// 텍스처 샘플러 설명자로부터 생성되며, 
/// 텍스처 샘플러 풀 객체에서 텍스처 샘플러를 찾을 때 사용됩니다.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SamplerID {
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

impl SamplerID {
    /// 주어진 텍스처 샘플러 설명자로부터 텍스처 샘플러 식별자를 생성합니다.
    #[must_use]
    fn new<'a>(desc: &wgpu::SamplerDescriptor<'a>) -> Self {
        // safety: 각 맴버의 크기가 같습니다.
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
/// 실제 풀 객체는 전역 변수로 선언되어 있으며, 
/// `SamplerPool`은 풀 객체에 접근할 수 있도록 합니다.
/// 
/// ※ 현재는 `blocking`으로 구현되어 있습니다.
/// 
#[derive(Debug)]
pub struct SamplerPool;

impl SamplerPool {
    /// 텍스처 샘플러를 가져옵니다.
    /// 생성된 텍스처 샘플러가 없는 경우 샘플러를 생성합니다.
    #[must_use]
    pub fn get_or_init<'a>(
        device: &wgpu::Device, 
        desc: &wgpu::SamplerDescriptor<'a>
    ) -> Arc<wgpu::Sampler> {
        // 텍스처 샘플러 식별자를 생성합니다.
        let id = SamplerID::new(desc);

        // 샘플러를 생성합니다. (임계 영역 최소화)
        let sampler = device.create_sampler(desc).into();

        {
            // 풀 객체의 lock을 획득합니다.
            let mut guard = POOL.lock().unwrap();

            // 풀 객체에 등록된 샘플러를 가져오거나 샘플러를 등록합니다.
            guard.entry(id)
                .or_insert(sampler)
                .clone()
        }
    }

    /// 주어진 텍스처 샘플러 설명자의 텍스처 샘플러를 풀 객체에서 제거합니다.
    pub fn remove<'a>(desc: &wgpu::SamplerDescriptor<'a>) {
        // 텍스처 샘플러 식별자를 생성합니다.
        let id = SamplerID::new(desc);

        {
            // 풀 객체의 lock을 획득합니다.
            let mut guard = POOL.lock().unwrap();

            // 풀 객체에 등록된 샘플러를 제거합니다.
            guard.remove(&id);
        }
    }

    /// 풀 객체를 초기화 합니다.
    pub fn clear() {
        {
            // 풀 객체의 lock을 획득합니다.
            let mut guard = POOL.lock().unwrap();

            // 풀 객체를 비웁니다.
            guard.clear();
        }
    }
}
