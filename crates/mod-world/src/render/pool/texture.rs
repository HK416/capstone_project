use std::{collections::HashMap, sync::{Arc, Mutex, OnceLock}};

use lazy_static::lazy_static;
use wgpu::util::DeviceExt;

use super::TextureViewPool;

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
    pub fn black(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
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
    pub fn white(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
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
    pub fn normal(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
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

    /// 기본 `Height` 텍스처를 반환합니다.
    #[must_use]
    pub fn height(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<wgpu::Texture> {
        static TEXTURE: OnceLock<Arc<wgpu::Texture>> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            device.create_texture_with_data(
                queue, 
                &wgpu::TextureDescriptor {
                    label: Some("DefaultHeightTexture"), 
                    size: wgpu::Extent3d {
                        width: 1, 
                        height: 1, 
                        depth_or_array_layers: 1
                    }, 
                    dimension: wgpu::TextureDimension::D2, 
                    format: wgpu::TextureFormat::R8Unorm, 
                    mip_level_count: 1, 
                    sample_count: 1, 
                    usage: wgpu::TextureUsages::TEXTURE_BINDING, 
                    view_formats: &[]
                }, 
                wgpu::util::TextureDataOrder::LayerMajor, 
                &[0]
            ).into()
        }).clone()
    }
}

impl TexturePool {
    /// 주어진 이름에 해당하는 텍스처를 가져옵니다.
    /// 만약 해당 텍스처가 풀 객체에 존재하지 않을 경우 텍스처를 생성합니다.
    #[inline]
    #[must_use]
    pub fn get_or_init<S, F>(name: S, func: F) -> Arc<wgpu::Texture> 
    where S: Into<String>, F: FnOnce() -> Arc<wgpu::Texture> {
        let mut lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.entry(name.into())
            .or_insert(func())
            .clone()
    }

    /// 주어진 이름에 해당하는 텍스처가 풀 객체에 포함되어있는지 여부를 반환합니다.
    #[inline]
    #[must_use]
    pub fn contains<S: AsRef<String>>(name: S) -> bool {
        let lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.contains_key(name.as_ref())
    }

    /// 주어진 이름에 해당하는 텍스처를 풀 객체에서 제거합니다.
    /// 만약 해당 텍스처가 풀 객체에 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    /// 
    /// 텍스처가 제거될 경우 텍스처 뷰도 자동으로 제거됩니다.
    /// 
    pub fn remove<S: AsRef<String>>(name: S) -> Option<Arc<wgpu::Texture>> {
        let mut lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        let result = lock_guard.remove(name.as_ref());
        drop(lock_guard);

        if let Some(texture) = result.as_ref() {
            TextureViewPool::remove(texture);
        }

        result
    }

    /// 풀 객체에 존재하는 모든 텍스처를 제거합니다.
    /// 
    /// 텍스처 뷰도 자동으로 제거됩니다.
    /// 
    pub fn clear() {
        let mut lock_guard = unsafe { POOL.lock().unwrap_unchecked() };
        lock_guard.clear();
        drop(lock_guard);

        TextureViewPool::clear();
    }
}
