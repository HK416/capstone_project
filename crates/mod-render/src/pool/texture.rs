use std::{
    error::Error,
    sync::{Arc, OnceLock},
};

use ahash::HashMap;
use parking_lot::{FairMutex, FairMutexGuard};
use wgpu::util::DeviceExt;

type PoolType = HashMap<String, Arc<wgpu::Texture>>;

/// 생성된 텍스처 객체를 관리하는 풀 객체입니다.
static POOL: OnceLock<FairMutex<PoolType>> = OnceLock::new();

/// 풀 객체를 가져옵니다.
fn get_pool() -> FairMutexGuard<'static, PoolType> {
    POOL.get_or_init(|| FairMutex::new(HashMap::default()))
        .lock()
}

/// ## Texture Pool  
/// 생성된 텍스처 객체를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `TexturePool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct TexturePool;

impl TexturePool {
    /// 미리 생성된 검정색 텍스처를 반환합니다.
    pub fn black(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
        let mut pool = get_pool();
        match pool.get("engine_generated_black").cloned() {
            Some(texture) => texture,
            None => {
                let texture = Arc::new(device.create_texture_with_data(
                    queue,
                    &wgpu::TextureDescriptor {
                        label: Some("Texture(engine_generated_black)"),
                        size: wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        mip_level_count: 1,
                        sample_count: 1,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &[0, 0, 0, 255],
                ));
                pool.insert("engine_generated_black".to_string(), texture.clone());
                texture
            }
        }
    }

    /// 미리 생성된 하얀색 텍스처를 반환합니다.
    pub fn white(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
        let mut pool = get_pool();
        match pool.get("engine_generated_white").cloned() {
            Some(texture) => texture,
            None => {
                let texture = Arc::new(device.create_texture_with_data(
                    queue,
                    &wgpu::TextureDescriptor {
                        label: Some("Texture(engine_generated_white)"),
                        size: wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        mip_level_count: 1,
                        sample_count: 1,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &[255, 255, 255, 255],
                ));
                pool.insert("engine_generated_black".to_string(), texture.clone());
                texture
            }
        }
    }

    /// 미리 생성된 노멀 텍스처를 반환합니다.
    pub fn normal(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
        let mut pool = get_pool();
        match pool.get("engine_generated_normal").cloned() {
            Some(texture) => texture,
            None => {
                let texture = Arc::new(device.create_texture_with_data(
                    queue,
                    &wgpu::TextureDescriptor {
                        label: Some("Texture(engine_generated_normal)"),
                        size: wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        mip_level_count: 1,
                        sample_count: 1,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &[127, 127, 127, 255],
                ));
                pool.insert("engine_generated_black".to_string(), texture.clone());
                texture
            }
        }
    }

    /// 미리 생성된 높이 텍스처를 반환합니다.
    pub fn height(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
        get_pool()
            .entry("Engine_Generated_Height".to_string())
            .or_insert(Arc::new(device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Engine_Generated_Height)"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::R8Unorm,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &[0],
            )))
            .clone()
    }
}

impl TexturePool {
    /// 주어진 Uri에 해당하는 텍스처 객체를 풀 객체에 등록합니다.  
    /// 이미 해당 Uri 텍스처가 존재하는 경우 기존의 텍스처를 반환합니다.
    pub fn register(uri: String, texture: Arc<wgpu::Texture>) -> Option<Arc<wgpu::Texture>> {
        get_pool().insert(uri, texture)
    }

    /// 주어진 Uri에 해당하는 텍스처 객체를 풀 객체에서 제거합니다.
    /// 해당 텍스처 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn unregister(uri: &str) -> Option<Arc<wgpu::Texture>> {
        get_pool().remove(uri)
    }

    /// 주어진 Uri에 해당하는 텍스처 객체를 풀 객체에서 가져옵니다.
    /// 해당 텍스처 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn get(uri: &str) -> Option<Arc<wgpu::Texture>> {
        get_pool().get(uri).cloned()
    }

    /// 이름에 해당하는 텍스처 객체를 가져옵니다.  
    /// 해당 텍스처 객체가 풀 객체에 존재하지 않는 경우 새로운 텍스처 객체를 생성합니다.
    pub fn get_or_init<F, E>(name: &str, func: F) -> Result<Arc<wgpu::Texture>, E>
    where
        F: FnOnce() -> Result<Arc<wgpu::Texture>, E>,
        E: Error + Send,
    {
        let mut pool = get_pool();
        match pool.get(name).cloned() {
            Some(texture) => Ok(texture),
            None => {
                let texture = func()?;
                pool.insert(name.to_string(), texture.clone());
                Ok(texture)
            }
        }
    }

    /// 이름에 해당하는 텍스처 객체가 풀 객체에 존재할 경우 `true`를 반환합니다.
    pub fn contains(name: &str) -> bool {
        get_pool().contains_key(name)
    }

    /// 풀 객체에 존재하는 모든 텍스처 객체를 제거합니다.
    pub fn clear() {
        get_pool().clear()
    }
}
