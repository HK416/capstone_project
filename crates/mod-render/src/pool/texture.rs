use std::sync::{Arc, OnceLock};

use ahash::RandomState;
use dashmap::DashMap;
use wgpu::util::DeviceExt;

/// 생성된 텍스처 객체를 관리하는 풀 객체입니다.
static POOL: OnceLock<DashMap<String, Arc<wgpu::Texture>, RandomState>> = OnceLock::new();

/// 풀 객체를 가져옵니다.
fn get_pool() -> &'static DashMap<String, Arc<wgpu::Texture>, RandomState> {
    POOL.get_or_init(|| DashMap::default())
}

/// ## Texture Pool  
/// 생성된 텍스처 객체를 관리하는 풀 객체입니다.  
/// 실제 풀 객체는 static 변수로 선언되어 있으며, `TexturePool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
pub struct TexturePool;

impl TexturePool {
    /// 미리 생성된 검정색 텍스처를 반환합니다.
    pub fn black(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
        get_pool()
            .entry("Engine_Generated_Black".to_string())
            .or_insert(Arc::new(device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Engine_Generated_Black)"),
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
            )))
            .clone()
    }

    /// 미리 생성된 하얀색 텍스처를 반환합니다.
    pub fn white(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
        get_pool()
            .entry("Engine_Generated_White".to_string())
            .or_insert(Arc::new(device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Engine_Generated_White)"),
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
            )))
            .clone()
    }

    /// 미리 생성된 노멀 텍스처를 반환합니다.
    pub fn normal(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
        get_pool()
            .entry("Engine_Generated_Normal".to_string())
            .or_insert(Arc::new(device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Engine_Generated_Normal)"),
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
            )))
            .clone()
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
    /// 이름에 해당하는 텍스처 객체를 가져옵니다.  
    /// 해당 텍스처 객체가 풀 객체에 존재하지 않는 경우 새로운 텍스처 객체를 생성합니다.
    pub fn get_or_init<S, F>(name: S, func: F) -> Arc<wgpu::Texture>
    where
        S: Into<String>,
        F: FnOnce() -> Arc<wgpu::Texture>,
    {
        get_pool().entry(name.into()).or_insert(func()).clone()
    }

    /// 이름에 해당하는 텍스처 객체가 풀 객체에 존재할 경우 `true`를 반환합니다.
    pub fn contains<S: AsRef<String>>(name: S) -> bool {
        get_pool().contains_key(name.as_ref())
    }

    /// 이름에 해당하는 텍스처 객체를 풀 객체에서 제거합니다.  
    /// 해당 텍스처 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove<S: AsRef<String>>(name: S) -> Option<Arc<wgpu::Texture>> {
        get_pool().remove(name.as_ref()).map(|(_, texture)| texture)
    }

    /// 풀 객체에 존재하는 모든 텍스처 객체를 제거합니다.
    pub fn clear() {
        get_pool().clear()
    }
}
