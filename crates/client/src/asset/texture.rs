#![allow(dead_code)]
//! 텍스처 에셋과 관련된 코드를 관리합니다.
//!

use std::{
    fs::OpenOptions,
    hash::{Hash, Hasher},
    io::Read,
    path::Path,
    sync::Arc,
};

use ahash::{AHasher, HashMap, RandomState};
use parking_lot::{FairMutex, FairMutexGuard};
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use super::AssetError;

/// 텍스처 포맷 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Bc4RUnorm,
    Bc5RgUnorm,
    Bc7RgbaUnorm,
    Bc7RgbaUnormSrgb,
}

impl Into<wgpu::TextureFormat> for TextureFormat {
    fn into(self) -> wgpu::TextureFormat {
        match self {
            TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureFormat::Bc4RUnorm => wgpu::TextureFormat::Bc4RUnorm,
            TextureFormat::Bc5RgUnorm => wgpu::TextureFormat::Bc5RgUnorm,
            TextureFormat::Bc7RgbaUnorm => wgpu::TextureFormat::Bc7RgbaUnorm,
            TextureFormat::Bc7RgbaUnormSrgb => wgpu::TextureFormat::Bc7RgbaUnormSrgb,
        }
    }
}

/// 텍스처 뷰 차원 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum TextureDimension {
    D1 = 0,
    D2 = 1,
    D2Array = 2,
    Cube = 3,
    CubeArray = 4,
    D3 = 5,
}

impl Into<wgpu::TextureDimension> for TextureDimension {
    fn into(self) -> wgpu::TextureDimension {
        match self {
            TextureDimension::D1 => wgpu::TextureDimension::D1,
            TextureDimension::D2 => wgpu::TextureDimension::D2,
            TextureDimension::D2Array => wgpu::TextureDimension::D2,
            TextureDimension::Cube => wgpu::TextureDimension::D2,
            TextureDimension::CubeArray => wgpu::TextureDimension::D2,
            TextureDimension::D3 => wgpu::TextureDimension::D3,
        }
    }
}

impl Into<wgpu::TextureViewDimension> for TextureDimension {
    fn into(self) -> wgpu::TextureViewDimension {
        match self {
            TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            TextureDimension::D2 => wgpu::TextureViewDimension::D2,
            TextureDimension::D2Array => wgpu::TextureViewDimension::D2Array,
            TextureDimension::Cube => wgpu::TextureViewDimension::Cube,
            TextureDimension::CubeArray => wgpu::TextureViewDimension::CubeArray,
            TextureDimension::D3 => wgpu::TextureViewDimension::D3,
        }
    }
}

/// 텍스처 샘플러의 주소 매핑 모드 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum TextureAddressMode {
    ClampToEdge = 0,
    Repeat = 1,
    MirrorRepeat = 2,
}

impl Into<wgpu::AddressMode> for TextureAddressMode {
    fn into(self) -> wgpu::AddressMode {
        match self {
            TextureAddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            TextureAddressMode::Repeat => wgpu::AddressMode::Repeat,
            TextureAddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        }
    }
}

/// 텍스처 샘플러의 필터링 모드 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum TextureFilterMode {
    Nearest = 0,
    Linear = 1,
}

impl Into<wgpu::FilterMode> for TextureFilterMode {
    fn into(self) -> wgpu::FilterMode {
        match self {
            TextureFilterMode::Nearest => wgpu::FilterMode::Nearest,
            TextureFilterMode::Linear => wgpu::FilterMode::Linear,
        }
    }
}

/// 텍스처 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TextureData {
    pub uri: String,
    pub width: u32,
    pub height: u32,
    pub depth_or_array_layers: u32,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub format: TextureFormat,
    pub dimension: TextureDimension,
    pub address_u: TextureAddressMode,
    pub address_v: TextureAddressMode,
    pub address_w: TextureAddressMode,
    pub filter_mode: TextureFilterMode,
}

/// 생성된 텍스처 데이터 객체를 관리하는 풀 객체입니다.
#[derive(Debug, Clone)]
pub struct TextureDataPool(Arc<FairMutex<TextureDataPoolType>>);

/// 텍스처 데이터 풀 객체의 타입입니다.
pub type TextureDataPoolType = HashMap<String, Arc<TextureData>>;

/// 텍스처 데이터 풀 객체의 용량입니다.
pub const TEXTURE_DATA_POOL_CAPACITY: usize = 128;

impl TextureDataPool {
    /// 새로운 풀 객체를 생성합니다.
    pub fn new() -> Self {
        Self(Arc::new(FairMutex::new(HashMap::with_capacity_and_hasher(
            TEXTURE_DATA_POOL_CAPACITY,
            RandomState::new(),
        ))))
    }

    /// 풀 객체의 `lock`을 획득합니다.
    ///
    /// # Warning
    /// `FairMutexGuard`가 지속되는 동안 풀 객체의 다른 함수를 호출하면 데드락이 발생합니다.
    ///
    pub fn lock(&self) -> FairMutexGuard<'_, TextureDataPoolType> {
        self.0.lock()
    }

    /// 파일로부터 [TextureData]를 생성합니다.
    fn load_from_file<Dir, Uri>(workspace: Dir, uri: Uri) -> Result<TextureData, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        let mut path = workspace.as_ref().to_path_buf();
        path.push(format!("{}.texD", uri.as_ref()));

        log::debug!("open texture data asset (PATH:{})", path.display());
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open texture data asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                AssetError::IOError(e)
            })?;

        log::debug!("read texture data asset (PATH:{})", path.display());
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read texture data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::IOError(e)
        })?;

        log::debug!("close texture data asset (PATH:{})", path.display());
        drop(file);

        log::debug!("decode texture data asset (PATH:{})", path.display());
        serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to decode texture data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::ParsingFailed(e)
        })
    }

    /// 텍스처 데이터 풀 객체에 등록된 텍스처를 가져옵니다.  
    /// 해당 Uri에 등록된 텍스처가 없는 경우 텍스처를 새로 생성합니다.
    pub fn get_or_init<Dir, Uri>(
        &self,
        workspace: Dir,
        uri: Uri,
    ) -> Result<Arc<TextureData>, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        // 풀 객체를 가져옵니다.
        let mut pool = self.lock();

        if let Some(texture) = pool.get(uri.as_ref()).cloned() {
            return Ok(texture);
        }

        // 텍스처 데이터를 생성합니다.
        let data = Arc::new(Self::load_from_file(workspace, uri)?);

        // 생성된 텍스처를 풀 객체에 등록합니다.
        pool.insert(data.uri.clone(), data.clone());
        Ok(data)
    }

    /// 텍스처 객체에 해당하는 텍스처 뷰 객체들을 풀 객체에서 제거합니다.  
    /// 해당 텍스처 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn get<Uri>(&self, uri: Uri) -> Option<Arc<TextureData>>
    where
        Uri: AsRef<str>,
    {
        self.lock().get(uri.as_ref()).cloned()
    }

    /// 텍스처 객체에 해당하는 텍스처 뷰 객체들을 풀 객체에서 제거합니다.  
    /// 해당 텍스처 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove<Uri>(&self, uri: Uri) -> Option<Arc<TextureData>>
    where
        Uri: AsRef<str>,
    {
        self.lock().remove(uri.as_ref()).map(|item| item)
    }

    /// 풀 객체에 존재하는 모든 텍스처 뷰 객체를 제거합니다.
    pub fn clear(&self) {
        self.lock().clear()
    }
}

#[derive(Debug, Clone)]
pub struct MipLevelCopyLayout {
    pub mip_level: u32,
    pub offset: u64,
    pub bytes_per_row: u32,
    pub rows_per_image: u32,
}

/// 생성된 텍스처 객체를 관리하는 풀 객체입니다.
#[derive(Debug, Clone)]
pub struct TexturePool(Arc<FairMutex<TexturePoolType>>);

/// 텍스처 풀 객체의 타입입니다.
pub type TexturePoolType = HashMap<String, Arc<wgpu::Texture>>;

/// 텍스처 풀 객체의 용량입니다.
pub const TEXTURE_POOL_CAPACITY: usize = 128;

impl TexturePool {
    /// 새로운 풀 객체를 생성합니다.
    pub fn new() -> Self {
        Self(Arc::new(FairMutex::new(HashMap::with_capacity_and_hasher(
            TEXTURE_POOL_CAPACITY,
            RandomState::new(),
        ))))
    }

    /// 풀 객체의 `lock`을 획득합니다.
    ///
    /// # Warning
    /// `FairMutexGuard`가 지속되는 동안 풀 객체의 다른 함수를 호출하면 데드락이 발생합니다.
    ///
    pub fn lock(&self) -> FairMutexGuard<'_, TexturePoolType> {
        self.0.lock()
    }

    /// 파일로부터 텍스처 데이터를 가져옵니다.
    fn load_from_file<Dir>(workspace: Dir, data: &TextureData) -> Result<Vec<u8>, AssetError>
    where
        Dir: AsRef<Path>,
    {
        use ddsfile::Dds;
        let mut path = workspace.as_ref().to_path_buf();
        path.push(format!("{}.dds", &data.uri));

        log::debug!("open texture pixel asset (PATH:{})", path.display());
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open texture pixel asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                AssetError::IOError(e)
            })?;

        log::debug!("read texture pixel asset (PATH:{})", path.display());
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read texture pixel asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::IOError(e)
        })?;

        log::debug!("close texture pixel asset (PATH:{})", path.display());
        drop(file);

        let dds = Dds::read(std::io::Cursor::new(buf)).unwrap();
        return Ok(dds.data);
    }

    /// 텍스처 포맷의 정보를 가져옵니다.
    fn get_texture_format_info(format: wgpu::TextureFormat) -> (bool, u32) {
        match format {
            wgpu::TextureFormat::Rgba8Unorm => (false, 4), // 비압축: 4 bytes / pixle
            wgpu::TextureFormat::Rgba8UnormSrgb => (false, 4), // 비압축: 4 bytes / pixle
            wgpu::TextureFormat::Bc4RUnorm => (true, 8),   // 압축: 8 bytes / 4x4 block
            wgpu::TextureFormat::Bc5RgUnorm => (true, 16), // 압축: 16 bytes / 4x4 block
            wgpu::TextureFormat::Bc7RgbaUnorm => (true, 16), // 압축: 16 bytes / 4x4 block
            wgpu::TextureFormat::Bc7RgbaUnormSrgb => (true, 16), // 압축: 16 bytes / 4x4 block
            _ => panic!("unsupported format!"),
        }
    }

    /// 패딩이 포함된 스테이징(업로드) 버퍼 데이터를 가져옵니다.
    fn get_staging_buffer_data_with_padding(
        width: u32,
        height: u32,
        mip_level_count: u32,
        is_compressed: bool,
        unit_bytes: u32,
        bytes: Vec<u8>,
    ) -> (Vec<u8>, Vec<MipLevelCopyLayout>) {
        let mut padded_data = Vec::new();
        let mut layout_info = Vec::new();

        // read_offset: 원본 데이터에서 읽어올 위치
        // write_offset: staging 데이터 내에서의 현재 위치 (패딩 포함)
        let mut read_offset = 0;
        let mut write_offset = 0;

        // 압축 텍스처의 경우 블록 크기 (BC 포맷은 일반적으로 4x4)
        let block_dim = if is_compressed { 4 } else { 1 };

        for mip_level in 0..mip_level_count {
            let mip_width = std::cmp::max(1, width >> mip_level);
            let mip_height = std::cmp::max(1, height >> mip_level);

            let (row_count, raw_bytes_per_row) = if is_compressed {
                // 블록 개수: (width+3)/4, (height+3)/4
                let block_width = (mip_width + block_dim - 1) / block_dim;
                let block_height = (mip_height + block_dim - 1) / block_dim;
                (block_height, block_width * unit_bytes)
            } else {
                (mip_height, mip_width * unit_bytes)
            };

            // WebGPU 요구사항: bytes_per_row는 256바이트 정렬이어야 함
            let padded_bytes_per_row = ((raw_bytes_per_row + 255) / 256) * 256;

            layout_info.push(MipLevelCopyLayout {
                mip_level,
                offset: write_offset as u64,
                bytes_per_row: padded_bytes_per_row as u32,
                rows_per_image: row_count,
            });

            for _ in 0..row_count {
                let start = read_offset;
                let end = start + raw_bytes_per_row;
                if end as usize > bytes.len() {
                    panic!(
                        "original bytes too small for mip level {} at read offset {}..{}",
                        mip_level, start, end
                    );
                }

                padded_data.extend_from_slice(&bytes[start as usize..end as usize]);
                padded_data.extend(
                    std::iter::repeat(0u8)
                        .take((padded_bytes_per_row - raw_bytes_per_row) as usize),
                );

                read_offset += raw_bytes_per_row;
                write_offset += padded_bytes_per_row;
            }
        }

        (padded_data, layout_info)
    }

    /// 주어진 데이터로 텍스처를 생성합니다.
    pub fn create_texture<Uri>(
        uri: Uri,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        width: u32,
        height: u32,
        depth_or_array_layers: u32,
        dimension: wgpu::TextureDimension,
        format: wgpu::TextureFormat,
        mip_level_count: u32,
        sample_count: u32,
        bytes: Vec<u8>,
    ) -> Arc<wgpu::Texture>
    where
        Uri: AsRef<str>,
    {
        let (is_compressed, unit_bytes) = Self::get_texture_format_info(format);
        let (padded_bytes, mip_layouts) = Self::get_staging_buffer_data_with_padding(
            width,
            height,
            mip_level_count,
            is_compressed,
            unit_bytes,
            bytes,
        );

        // 스테이징 버퍼를 생성합니다.
        log::debug!("create staging buffer (URI:{})", uri.as_ref());
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Staging(Texture({}))", uri.as_ref())),
            contents: &padded_bytes,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // 텍스처를 생성합니다.
        log::debug!("create texture (URI:{})", uri.as_ref());
        let texture = Arc::new(device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Texture({})", uri.as_ref())),
            dimension,
            format,
            mip_level_count,
            sample_count,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers,
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        }));

        // 압축 텍스처는 블록 단위로 copy extent가 지정되어야 합니다.
        let block_dim = if is_compressed { 4 } else { 1 };

        for layout in mip_layouts {
            // 각 밉 레벨 실제 크기
            let mip_width = std::cmp::max(1, width >> layout.mip_level);
            let mip_height = std::cmp::max(1, height >> layout.mip_level);

            // 압축 텍스처의 경우, copy extent의 width, height는 블록 단위로 올림합니다.
            let copy_width = if is_compressed {
                std::cmp::max(1, ((mip_width + block_dim - 1) / block_dim) * block_dim)
            } else {
                mip_width
            };

            let copy_height = if is_compressed {
                std::cmp::max(1, ((mip_height + block_dim - 1) / block_dim) * block_dim)
            } else {
                mip_height
            };

            encoder.copy_buffer_to_texture(
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: layout.offset,
                        bytes_per_row: Some(layout.bytes_per_row),
                        rows_per_image: Some(layout.rows_per_image),
                    },
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: layout.mip_level,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: copy_width,
                    height: copy_height,
                    depth_or_array_layers: 1,
                },
            );
        }

        staging_buffers.push(buffer);
        texture
    }

    /// 텍스처 풀 객체에 등록된 텍스처를 가져옵니다.  
    /// 해당 Uri에 등록된 텍스처가 없는 경우 텍스처를 새로 생성합니다.
    pub fn get_or_init<Dir>(
        &self,
        workspace: Dir,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging_buffers: &mut Vec<wgpu::Buffer>,
        data: &TextureData,
    ) -> Result<Arc<wgpu::Texture>, AssetError>
    where
        Dir: AsRef<Path>,
    {
        // 풀 객체를 가져옵니다.
        let mut pool = self.lock();

        if let Some(texture) = pool.get(&data.uri).cloned() {
            return Ok(texture);
        }

        // 텍스처를 생성합니다.
        let bytes = Self::load_from_file(workspace, data)?;
        let texture = Self::create_texture(
            &data.uri,
            device,
            encoder,
            staging_buffers,
            data.width,
            data.height,
            data.depth_or_array_layers,
            data.dimension.into(),
            data.format.into(),
            data.mip_level_count,
            data.sample_count,
            bytes,
        );

        // 생성된 텍스처를 풀 객체에 등록합니다.
        pool.insert(data.uri.clone(), texture.clone());
        Ok(texture)
    }

    /// 텍스처 풀 객체에 텍스처를 등록합니다.  
    /// 이미 Uri에 해당하는 텍스처가 존재할 경우 기존의 텍스처를 반환합니다.
    pub fn insert<Uri>(&self, uri: Uri, texture: Arc<wgpu::Texture>) -> Option<Arc<wgpu::Texture>>
    where
        Uri: AsRef<str>,
    {
        self.lock().insert(uri.as_ref().into(), texture)
    }

    /// Uri에 해당하는 텍스처 객체를 가져옵니다.  
    /// 해당 텍스처 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn get<Uri>(&self, uri: Uri) -> Option<Arc<wgpu::Texture>>
    where
        Uri: AsRef<str>,
    {
        self.lock().get(uri.as_ref()).cloned()
    }

    /// Uri에 해당하는 텍스처 객체들를 풀 객체에서 제거합니다.  
    /// 텍스처 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove<Uri>(&self, uri: Uri) -> Option<Arc<wgpu::Texture>>
    where
        Uri: AsRef<str>,
    {
        self.lock().remove(uri.as_ref()).map(|item| item)
    }

    /// 풀 객체에 존재하는 모든 텍스처 객체를 제거합니다.
    pub fn clear(&self) {
        self.lock().clear()
    }
}

/// 생성된 텍스처 뷰 객체를 관리하는 풀 객체입니다.
#[derive(Debug, Clone)]
pub struct TextureViewPool(Arc<FairMutex<TextureViewPoolType>>);

/// 텍스처 뷰 풀 객체의 타입입니다.
pub type TextureViewPoolType = HashMap<Arc<wgpu::Texture>, HashMap<u64, Arc<wgpu::TextureView>>>;

/// 텍스처 뷰 풀 객체의 용량입니다.
pub const TEXTURE_VIEW_POOL_CAPACITY: usize = 128;

impl TextureViewPool {
    /// 새로운 풀 객체를 생성합니다.
    pub fn new() -> Self {
        Self(Arc::new(FairMutex::new(HashMap::with_capacity_and_hasher(
            TEXTURE_VIEW_POOL_CAPACITY,
            RandomState::new(),
        ))))
    }

    /// 풀 객체의 `lock`을 획득합니다.
    ///
    /// # Warning
    /// `FairMutexGuard`가 지속되는 동안 풀 객체의 다른 함수를 호출하면 데드락이 발생합니다.
    ///
    pub fn lock(&self) -> FairMutexGuard<'_, TextureViewPoolType> {
        self.0.lock()
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

    /// 텍스처 객체와 설명자에 해당하는 텍스처 뷰 객체를 가져옵니다.  
    /// 해당 텍스처 뷰 객체가 풀 객체에 존재하지 않는 경우 새로운 텍스처 뷰 객체를 생성합니다.
    pub fn get_or_init(
        &self,
        texture: &Arc<wgpu::Texture>,
        desc: &wgpu::TextureViewDescriptor,
    ) -> Arc<wgpu::TextureView> {
        let key = Self::get_hash(desc);
        let mut pool = self.lock();
        match pool.get(texture).cloned() {
            Some(mut map) => match map.get(&key).cloned() {
                Some(view) => view,
                None => {
                    let view = Arc::new(texture.create_view(desc));
                    map.insert(key, view.clone());
                    view
                }
            },
            None => {
                let mut map = HashMap::default();
                let view = Arc::new(texture.create_view(desc));
                map.insert(key, view.clone());
                pool.insert(texture.clone(), map);
                view
            }
        }
    }

    /// 텍스처 객체에 해당하는 텍스처 뷰 객체들을 풀 객체에서 제거합니다.  
    /// 해당 텍스처 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove(&self, texture: &Arc<wgpu::Texture>) -> Option<Vec<Arc<wgpu::TextureView>>> {
        self.lock()
            .remove(texture)
            .map(|pool| pool.into_values().collect())
    }

    /// 풀 객체에 존재하는 모든 텍스처 뷰 객체를 제거합니다.
    pub fn clear(&self) {
        self.lock().clear()
    }
}

/// 생성된 텍스처 샘플러 객체를 관리하는 풀 객체입니다.  
#[derive(Debug, Clone)]
pub struct SamplerPool(Arc<FairMutex<SamplerPoolType>>);

/// 텍스처 샘플러 풀 객체의 타입입니다.
type SamplerPoolType = HashMap<u64, Arc<wgpu::Sampler>>;

/// 샘플러 풀 객체의 용량입니다.
pub const SAMPLER_POOL_CAPACITY: usize = 16;

impl SamplerPool {
    /// 새로운 풀 객체를 생성합니다.
    pub fn new() -> Self {
        Self(Arc::new(FairMutex::new(HashMap::with_capacity_and_hasher(
            SAMPLER_POOL_CAPACITY,
            RandomState::new(),
        ))))
    }

    /// 풀 객체의 `lock`을 획득합니다.
    ///
    /// # Warning
    /// `FairMutexGuard`가 지속되는 동안 풀 객체의 다른 함수를 호출하면 데드락이 발생합니다.
    ///
    pub fn lock(&self) -> FairMutexGuard<'_, SamplerPoolType> {
        self.0.lock()
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

    /// 설명자에 해당하는 텍스처 샘플러 객체를 가져옵니다.  
    /// 해당 텍스처 샘플러 객체가 풀 객체에 존재하지 않는 경우 새로운 텍스처 샘플러 객체를 생성합니다.
    pub fn get_or_init(
        &self,
        device: &wgpu::Device,
        desc: &wgpu::SamplerDescriptor,
    ) -> Arc<wgpu::Sampler> {
        let key = Self::get_hash(desc);
        let mut pool = self.lock();
        match pool.get(&key).cloned() {
            Some(sampler) => sampler,
            None => {
                let sampler = Arc::new(device.create_sampler(desc));
                pool.insert(key, sampler.clone());
                sampler
            }
        }
    }

    /// 설명자에 해당하는 텍스처 샘플러 객체를 풀 객체에서 제거합니다.  
    /// 해당 텍스처 샘플러 객체가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove(&self, desc: &wgpu::SamplerDescriptor) -> Option<Arc<wgpu::Sampler>> {
        self.lock().remove(&Self::get_hash(desc))
    }

    /// 풀 객체에 존재하는 모든 텍스처 샘플러 객체를 제거합니다.
    pub fn clear(&self) {
        self.lock().clear()
    }
}
