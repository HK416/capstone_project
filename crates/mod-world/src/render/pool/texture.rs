use std::{collections::HashMap, sync::{Arc, Mutex, OnceLock}};

use lazy_static::lazy_static;
use wgpu::util::DeviceExt;

lazy_static! {
    /// 생성된 텍스처를 관리하는 풀 객체입니다.
    static ref POOL: Mutex<HashMap<String, Arc<wgpu::Texture>>> = Mutex::new(HashMap::with_capacity(32));
}



/// 생성된 텍스처를 관리하는 풀 객체입니다.
/// 
/// 실제 풀 객체는 전역 변수로 선언되어 있으며, 
/// `TexturePool`은 풀 객체에 접근할 수 있도록 합니다.
/// 
pub struct TexturePool;

impl TexturePool {
    /// 기본 검정색 텍스처를 반환합니다.
    #[must_use]
    pub fn black(device: &Arc<wgpu::Device>, queue: &Arc<wgpu::Queue>) -> Arc<wgpu::Texture> {
        static TEXTURE: OnceLock<Arc<wgpu::Texture>> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            device.create_texture_with_data(
                queue, 
                &wgpu::TextureDescriptor {
                    label: Some("DefaultBlackTexture"), 
                    size: wgpu::Extent3d {
                        width: 1, 
                        height: 1, 
                        depth_or_array_layers: 1
                    }, 
                    dimension: wgpu::TextureDimension::D2, 
                    format: wgpu::TextureFormat::Rgba8Unorm, 
                    mip_level_count: 1, 
                    sample_count: 1, 
                    usage: wgpu::TextureUsages::TEXTURE_BINDING, 
                    view_formats: &[]
                }, 
                wgpu::util::TextureDataOrder::LayerMajor, 
                &[0, 0, 0, 255]
            ).into()
        }).clone()
    }

    /// 기본 하얀색 텍스처를 반환합니다.
    #[must_use]
    pub fn white(device: &Arc<wgpu::Device>, queue: &Arc<wgpu::Queue>) -> Arc<wgpu::Texture> {
        static TEXTURE: OnceLock<Arc<wgpu::Texture>> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            device.create_texture_with_data(
                queue, 
                &wgpu::TextureDescriptor {
                    label: Some("DefaultWhiteTexture"), 
                    size: wgpu::Extent3d {
                        width: 1, 
                        height: 1, 
                        depth_or_array_layers: 1
                    }, 
                    dimension: wgpu::TextureDimension::D2, 
                    format: wgpu::TextureFormat::Rgba8Unorm, 
                    mip_level_count: 1, 
                    sample_count: 1, 
                    usage: wgpu::TextureUsages::TEXTURE_BINDING, 
                    view_formats: &[]
                }, 
                wgpu::util::TextureDataOrder::LayerMajor, 
                &[255, 255, 255, 255]
            ).into()
        }).clone()
    }

    /// 기본 노멀 텍스처를 반환합니다.
    #[must_use]
    pub fn normal(device: &Arc<wgpu::Device>, queue: &Arc<wgpu::Queue>) -> Arc<wgpu::Texture> {
        static TEXTURE: OnceLock<Arc<wgpu::Texture>> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            device.create_texture_with_data(
                queue, 
                &wgpu::TextureDescriptor {
                    label: Some("DefaultNormalTexture"), 
                    size: wgpu::Extent3d {
                        width: 1, 
                        height: 1, 
                        depth_or_array_layers: 1
                    }, 
                    dimension: wgpu::TextureDimension::D2, 
                    format: wgpu::TextureFormat::Rgba8Unorm, 
                    mip_level_count: 1, 
                    sample_count: 1, 
                    usage: wgpu::TextureUsages::TEXTURE_BINDING, 
                    view_formats: &[]
                }, 
                wgpu::util::TextureDataOrder::LayerMajor, 
                &[127, 127, 127, 255]
            ).into()
        }).clone()
    }
}

impl TexturePool {
    /// 텍스처를 가져옵니다.
    /// 만약 텍스처가 풀 객체에 존재하지 않을 경우 텍스처를 생성합니다.
    #[must_use]
    pub fn get_or_init<'a, N: Into<String>>(
        device: &Arc<wgpu::Device>, 
        queue: &Arc<wgpu::Queue>, 
        texture_name: N, 
        desc: &wgpu::TextureDescriptor<'a>, 
        data: &[u8]
    ) -> Arc<wgpu::Texture> {
        let texture_name = texture_name.into();
        let mut pool_guard = POOL.lock().unwrap();
        match pool_guard.get(&texture_name) {
            Some(texture) => texture.clone(), 
            None => {
                let texture = Arc::new(device.create_texture_with_data(
                    queue, 
                    desc, 
                    wgpu::util::TextureDataOrder::LayerMajor, 
                    data
                ));
                pool_guard.insert(texture_name, texture.clone());
                texture
            }
        }
    }

    /// 주어진 텍스처에 해당하는 텍스처를 풀 객체에서 가져옵니다.
    /// 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    #[must_use]
    pub fn get<N: AsRef<str>>(texture_name: N) -> Option<Arc<wgpu::Texture>> {
        let pool_guard = POOL.lock().unwrap();
        pool_guard.get(texture_name.as_ref()).cloned()
    }

    /// 주어진 텍스처에 해당하는 텍스처가 풀 객체에 포함되어있는지 여부를 반환합니다.
    #[must_use]
    pub fn contains<N: AsRef<str>>(texture_name: N) -> bool {
        let pool_guard = POOL.lock().unwrap();
        pool_guard.contains_key(texture_name.as_ref())
    }

    /// 주어진 텍스처에 해당하는 텍스처를 풀 객체에서 제거합니다.
    pub fn remove<N: AsRef<str>>(texture_name: N) -> Option<Arc<wgpu::Texture>> {
        let mut pool_guard = POOL.lock().unwrap();
        pool_guard.remove(texture_name.as_ref())
    }

    /// 풀 객체에 존재하는 모든 텍스처를 제거합니다.
    pub fn clear() {
        let mut pool_guard = POOL.lock().unwrap();
        pool_guard.clear();
    }
}
