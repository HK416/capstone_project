use std::{collections::HashMap, mem, sync::{Arc, Mutex}};

use lazy_static::lazy_static;

lazy_static! {
    /// 생성된 텍스처 뷰를 관리하는 풀 객체입니다.
    static ref POOL: Mutex<HashMap<TextureViewID, Arc<wgpu::TextureView>>> = Mutex::new(HashMap::with_capacity(32));
}



/// 텍스처 뷰 풀 객체에서 텍스처 뷰를 식별하기 위한 식별자입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureViewID {
    global_id: wgpu::Id<wgpu::Texture>, 
    format: [u8; mem::size_of::<Option<wgpu::TextureFormat>>()], 
    dimension: [u8; mem::size_of::<Option<wgpu::TextureViewDimension>>()], 
    aspect: [u8; mem::size_of::<wgpu::TextureAspect>()], 
    base_mip_level: [u8; mem::size_of::<u32>()], 
    mip_level_count: [u8; mem::size_of::<Option<u32>>()], 
    base_array_layer: [u8; mem::size_of::<u32>()], 
    array_layer_count: [u8; mem::size_of::<Option<u32>>()], 
}

impl From<(&wgpu::Texture, &wgpu::TextureViewDescriptor<'_>)> for TextureViewID {
    fn from((texture, desc): (&wgpu::Texture, &wgpu::TextureViewDescriptor<'_>)) -> Self {
        // Safe: 각 맴버의 크기는 같습니다.
        unsafe {
            Self {
                global_id: texture.global_id(), 
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

impl From<(&Arc<wgpu::Texture>, &wgpu::TextureViewDescriptor<'_>)> for TextureViewID {
    fn from((texture, desc): (&Arc<wgpu::Texture>, &wgpu::TextureViewDescriptor<'_>)) -> Self {
        // Safe: 각 맴버의 크기는 같습니다.
        unsafe {
            Self {
                global_id: texture.global_id(), 
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
    /// 텍스처 뷰를 가져옵니다.
    /// 만약 텍스처 뷰가 풀 객체에 존재하지 않을 경우 텍스처 뷰를 생성합니다.
    #[must_use]
    pub fn get_or_init<'a>(
        texture: &Arc<wgpu::Texture>, 
        desc: &wgpu::TextureViewDescriptor<'a>
    ) -> Arc<wgpu::TextureView> {
        let texture_view_id = TextureViewID::from((texture, desc));
        let mut pool_guard = POOL.lock().unwrap();
        match pool_guard.get(&texture_view_id) {
            Some(texture_view) => texture_view.clone(), 
            None => {
                let texture_view = Arc::new(texture.create_view(desc));
                pool_guard.insert(texture_view_id, texture_view.clone());
                texture_view
            }
        }
    }

    /// 주어진 텍스처 뷰에 해당하는 텍스처 뷰가 풀 객체에 포함되어있는지 여부를 반환합니다.
    #[must_use]
    pub fn contains(texture_view_id: &TextureViewID) -> bool {
        let pool_guard = POOL.lock().unwrap();
        pool_guard.contains_key(texture_view_id)
    }

    /// 주어진 텍스처 뷰에 해당하는 텍스처 뷰를 풀 객체에서 제거합니다.
    pub fn remove(texture_view_id: &TextureViewID) -> Option<Arc<wgpu::TextureView>> {
        let mut pool_guard = POOL.lock().unwrap();
        pool_guard.remove(texture_view_id)
    }

    /// 풀 객체에 존재하는 모든 텍스처 뷰를 제거합니다.
    pub fn clear() {
        let mut pool_guard = POOL.lock().unwrap();
        pool_guard.clear();
    }
}
