use std::{collections::HashMap, mem, sync::{Arc, Mutex}};

use lazy_static::lazy_static;

lazy_static! {
    /// 생성된 텍스처 뷰를 관리하는 풀 객체입니다.
    static ref POOL: Mutex<HashMap<wgpu::Id<wgpu::Texture>, HashMap<TextureViewDescriptorId, Arc<wgpu::TextureView>>>> = Mutex::new(HashMap::with_capacity(32));
}



/// 텍스처 뷰 풀 객체에서 텍스처 뷰를 식별하기 위한 식별자입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TextureViewDescriptorId {
    format: [u8; mem::size_of::<Option<wgpu::TextureFormat>>()], 
    dimension: [u8; mem::size_of::<Option<wgpu::TextureViewDimension>>()], 
    aspect: [u8; mem::size_of::<wgpu::TextureAspect>()], 
    base_mip_level: [u8; mem::size_of::<u32>()], 
    mip_level_count: [u8; mem::size_of::<Option<u32>>()], 
    base_array_layer: [u8; mem::size_of::<u32>()], 
    array_layer_count: [u8; mem::size_of::<Option<u32>>()], 
}

impl From<&wgpu::TextureViewDescriptor<'_>> for TextureViewDescriptorId {
    fn from(desc: &wgpu::TextureViewDescriptor<'_>) -> Self {
        // Safe: 각 맴버의 크기는 같습니다.
        unsafe {
            Self {
                format: mem::transmute_copy(&desc.format), 
                dimension: mem::transmute_copy(&desc.dimension), 
                aspect: mem::transmute_copy(&desc.aspect), 
                base_mip_level: mem::transmute_copy(&desc.base_mip_level), 
                mip_level_count: mem::transmute_copy(&desc.mip_level_count), 
                base_array_layer: mem::transmute_copy(&desc.base_array_layer), 
                array_layer_count: mem::transmute_copy(&desc.array_layer_count), 
            }
        }
    }
}



/// 생성된 텍스처 뷰를 관리하는 풀 객체입니다.
/// 
/// 실제 풀 객체는 전역 변수로 선언되어있으며, 
/// `TextureViewPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
/// 
pub struct TextureViewPool;

impl TextureViewPool {
    /// 주어진 텍스처 뷰 설명자에 해당하는 텍스처 뷰를 가져옵니다.
    /// 만약 텍스처 뷰가 풀 객체에 존재하지 않을 경우 텍스처 뷰를 생성합니다.
    #[inline]
    #[must_use]
    pub fn get_or_init<'a>(
        texture: &wgpu::Texture, 
        desc: &wgpu::TextureViewDescriptor<'a>
    ) -> Arc<wgpu::TextureView> {
        let descriptor_id = TextureViewDescriptorId::from(desc);
        let mut lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.entry(texture.global_id())
            .or_default()
            .entry(descriptor_id)
            .or_insert(Arc::new(texture.create_view(desc)))
            .clone()
    }

    /// 주어진 텍스처 뷰 설명자에 해당하는 텍스처 뷰가 풀 객체에 포함되어있는지 여부를 반환합니다.
    #[inline]
    #[must_use]
    pub fn contains<'a>(texture: &wgpu::Texture, desc: &wgpu::TextureViewDescriptor<'a>) -> bool {
        let descriptor_id = TextureViewDescriptorId::from(desc);
        let lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.get(&texture.global_id())
            .map(|pool| {
                pool.get(&descriptor_id)
            })
            .flatten()
            .is_some()
    }

    /// 주어진 텍스처에 해당하는 모든 텍스처 뷰를 풀 객체에서 제거합니다.
    /// 만약 해당 텍스처가 풀 객체에 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    #[inline]
    pub fn remove(texture: &wgpu::Texture) {
        let mut lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.remove(&texture.global_id());
    }

    /// 풀 객체에 존재하는 모든 텍스처 뷰를 제거합니다.
    #[inline]
    pub fn clear() {
        let mut lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.clear();
    }
}
