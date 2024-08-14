use std::mem;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use hashbrown::HashMap;
use wgpu::util::DeviceExt;
use lazy_static::lazy_static;

lazy_static! {
    /// 생성된 텍스처를 관리하는 풀 객체입니다.
    static ref POOL: Mutex<HashMap<TextureID, Arc<TextureHandle>>> = Mutex::new(HashMap::with_capacity(32));
}



/// 텍스처의 식별자입니다.
/// 텍스처 파일의 이름 또는 경로로부터 생성되며, 
/// 텍스처 풀 객체에서 텍스처를 찾을 때 사용됩니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureID([char; 64]);

impl From<&Path> for TextureID {
    #[inline]
    fn from(value: &Path) -> Self {
        let path = value.to_string_lossy().to_string();
        let mut chars = path.chars();
        let buffer = ['\0'; 64]
            .map(|def| chars.next().unwrap_or(def));
        Self(buffer)
    }
}

impl From<PathBuf> for TextureID {
    #[inline]
    fn from(value: PathBuf) -> Self {
        let path = value.to_string_lossy().to_string();
        let mut chars = path.chars();
        let buffer = ['\0'; 64]
            .map(|def| chars.next().unwrap_or(def));
        Self(buffer)
    }
}

impl From<&str> for TextureID {
    #[inline]
    fn from(value: &str) -> Self {
        let s = value.to_string();
        let mut chars = s.chars();
        let buffer = ['\0'; 64]
            .map(|def| chars.next().unwrap_or(def));
        Self(buffer)
    }
}

impl From<String> for TextureID {
    #[inline]
    fn from(value: String) -> Self {
        let mut chars = value.chars();
        let buffer = ['\0'; 64]
            .map(|def| chars.next().unwrap_or(def));
        Self(buffer)
    }
}



/// 텍스처 뷰의 식별자입니다.
/// 텍스처 뷰 설명자로부터 생성되며, 
/// 각 텍스처에 있는 풀 객체에서 텍스처 뷰를 찾을 때 사용됩니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TextureViewID {
    format: [u8; mem::size_of::<Option<wgpu::TextureFormat>>()], 
    dimension: [u8; mem::size_of::<Option<wgpu::TextureViewDimension>>()], 
    aspect: [u8; mem::size_of::<wgpu::TextureAspect>()], 
    base_mip_level: [u8; mem::size_of::<u32>()], 
    mip_level_count: [u8; mem::size_of::<Option<u32>>()], 
    base_array_layer: [u8; mem::size_of::<u32>()], 
    array_layer_count: [u8; mem::size_of::<Option<u32>>()], 
}

impl TextureViewID {
    /// 주어진 텍스처 뷰 설명자로부터 텍스처 뷰 식별자를 생성합니다.
    #[must_use]
    pub fn new<'a>(desc: &wgpu::TextureViewDescriptor<'a>) -> Self {
        // safety: 각 맴버의 크기가 같습니다.
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



/// 생성된 텍스처의 제어 핸들입니다.
#[derive(Debug)]
pub struct TextureHandle {
    /// 이미지 픽셀 데이터를 가진 텍스처 객체입니다.
    texture: wgpu::Texture, 
    /// 쉐이더에서 엑세스 가능한 텍스처 뷰 객체 풀입니다.
    /// 
    /// ※ 현재는 `blocking`으로 구현되어 있습니다.
    /// 
    texture_views: Mutex<HashMap<TextureViewID, Arc<wgpu::TextureView>>>, 
}

impl TextureHandle {
    /// 초기화 되지 않은 텍스처를 생성합니다.
    #[must_use]
    fn new<'a>(
        device: &wgpu::Device, 
        desc: &wgpu::TextureDescriptor<'a>
    ) -> Self {
        Self {
            texture: device.create_texture(desc), 
            texture_views: Mutex::new(HashMap::with_capacity(4)), 
        }
    }

    /// 초기화된 텍스처를 생성합니다.
    #[must_use]
    fn new_with_data<'a>(
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        desc: &wgpu::TextureDescriptor<'a>, 
        order: wgpu::util::TextureDataOrder, 
        data: &[u8]
    ) -> Self {
        Self { 
            texture: device.create_texture_with_data(queue, desc, order, data), 
            texture_views: Mutex::new(HashMap::with_capacity(4)) 
        }
    }

    /// 텍스처 전체를 나타내는 [wgpu::ImageCopyTexture]를 만듭니다.
    #[inline]
    pub fn as_image_copy<'a>(&'a self) -> wgpu::ImageCopyTexture<'a> {
        self.texture.as_image_copy()
    }

    /// 텍스처의 크기를 반환합니다.
    #[inline]
    pub fn size(&self) -> wgpu::Extent3d {
        self.texture.size()
    }

    /// 텍스처의 가로 길이를 반환합니다.
    #[inline]
    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    /// 텍스처의 세로 길이를 반환합니다.
    #[inline]
    pub fn height(&self) -> u32 {
        self.texture.height()
    }

    /// 텍스처의 깊이 또는 레이어 갯수를 반환합니다.
    #[inline]
    pub fn depth_or_array_layers(&self) -> u32 {
        self.texture.depth_or_array_layers()
    }

    /// 텍스처의 밉맵 레벨의 갯수를 반환합니다.
    #[inline]
    pub fn mip_level_count(&self) -> u32 {
        self.texture.mip_level_count()
    }

    /// 텍스처의 샘플 갯수를 반환합니다.
    #[inline]
    pub fn sample_count(&self) -> u32 {
        self.texture.sample_count()
    }

    /// 텍스처의 차원을 반환합니다.
    #[inline]
    pub fn dimension(&self) -> wgpu::TextureDimension {
        self.texture.dimension()
    }

    /// 텍스처의 [wgpu::TextureUsages]를 반환합니다.
    #[inline]
    pub fn usage(&self) -> wgpu::TextureUsages {
        self.texture.usage()
    }

    /// 텍스처의 글로벌 식별자를 반환합니다.
    #[inline]
    pub fn global_id(&self) -> wgpu::Id<wgpu::Texture> {
        self.texture.global_id()
    }

    /// 텍스처에 데이터를 쓰기 명령을 명령 대기열에 추가합니다.
    /// ※ 데이터가 텍스처에 바로 반영되지 않습니다.
    /// 
    /// - `target_mip_level`: 텍스처의 대상 밉맵 레벨입니다.
    /// - `target_origin`: 선택한 밉맵 레벨에 있는 텍셀 단위의 시작 위치입니다.
    /// - `target_aspect`: 텍스처가 보유하는 데이터의 종류입니다.
    /// - `data`: 텍셀 데이터가 포함된 쓰기 데이터입니다. 텍스처와 동일한 형식이어야 합니다.
    /// - `data_layout`: 쓰기 데이터의 메모리 레이아웃을 설명합니다.
    /// - `size`: 텍셀 단위의 쓰기 영역의 크기입니다.
    /// 
    /// 자세한 내용은 [wgpu::Queue] 문서를 확인하세요.
    #[inline]
    pub fn write_texture(
        &self, 
        queue: &wgpu::Queue, 
        target_mip_level: u32, 
        target_origin: wgpu::Origin3d, 
        target_aspect: wgpu::TextureAspect, 
        data: &[u8], 
        data_layout: wgpu::ImageDataLayout, 
        size: wgpu::Extent3d
    ) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture, 
                mip_level: target_mip_level, 
                origin: target_origin, 
                aspect: target_aspect, 
            }, 
            data, 
            data_layout, 
            size
        )
    }

    /// 텍스처 뷰를 가져옵니다.
    /// 생성된 텍스처 뷰가 없는 경우 텍스처 뷰를 등록합니다.
    pub fn get_view_or_init<'a>(&self, desc: &wgpu::TextureViewDescriptor<'a>) -> Arc<wgpu::TextureView> {
        // 텍스처 뷰 식별자를 생성합니다.
        let id = TextureViewID::new(desc);

        // 텍스처 뷰를 생성합니다. (임계 영역 최소화)
        let texture_view: Arc<_> = self.texture.create_view(desc).into();

        {
            // 풀 객체의 lock을 획득합니다.
            let mut guard = self.texture_views.lock().unwrap();

            // 풀 객체에 등록된 텍스처 뷰를 가져오거나 등록합니다.
            guard.entry(id).or_insert(texture_view).clone()
        }
    }

    /// 주어진 텍스처 뷰 설명자에 해당하는 텍스처 뷰를 풀 객체에서 제거합니다.
    pub fn remove<'a>(&self, desc: &wgpu::TextureViewDescriptor<'a>) {
        // 텍스처 뷰 식별자를 생성합니다.
        let id = TextureViewID::new(desc);

        {
            // 풀 객체의 lock을 획득합니다.
            let mut guard = self.texture_views.lock().unwrap();

            // 풀 객체에 등록된 텍스처 뷰를 제거합니다.
            guard.remove(&id);
        }
    }

    /// 풀 객체를 초기화 합니다.
    pub fn view_clear(&self) {
        {
            // 풀 객체의 lock을 획득합니다.
            let mut guard = self.texture_views.lock().unwrap();

            // 풀 객체를 비웁니다.
            guard.clear();
        }
    }
}



/// 생성된 텍스처를 관리하는 풀 객체입니다.
/// 
/// 실제 풀 객체는 전역 변수로 선언되어 있으며, 
/// `TexturePool`은 풀 객체에 접근할 수 있도록 합니다.
/// 
/// ※ 현재는 `blocking`으로 구현되어 있습니다.
/// 
#[derive(Debug)]
pub struct TexturePool;

impl TexturePool {
    /// 검정색 텍스처를 반환합니다.
    pub fn black(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<TextureHandle> {
        static TEXTURE: OnceLock<Arc<TextureHandle>> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            TexturePool::spawn_with_data(
                device, 
                queue, 
                "Default(Black)", 
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Default(Black))"), 
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
            )
        }).clone()
    }

    /// 하얀색 텍스처를 반환합니다.
    pub fn white(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<TextureHandle> {
        static TEXTURE: OnceLock<Arc<TextureHandle>> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            TexturePool::spawn_with_data(
                device, 
                queue, 
                "Default(White)", 
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Default(White))"), 
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
            )
        }).clone()
    }

    /// 기본 노멀 텍스처를 반환합니다.
    pub fn normal(device: &wgpu::Device, queue: &wgpu::Queue) -> Arc<TextureHandle> {
        static TEXTURE: OnceLock<Arc<TextureHandle>> = OnceLock::new();
        TEXTURE.get_or_init(|| {
            TexturePool::spawn_with_data(
                device, 
                queue, 
                "Default(Normal)", 
                &wgpu::TextureDescriptor {
                    label: Some("Texture(Default(Normal))"), 
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
            )
        }).clone()
    }
}

impl TexturePool {
    /// 초기화 되지 않은 텍스처를 생성한 후, 주어진 텍스처 식별자로 풀 객체에 등록합니다.
    /// 
    /// 이미 주어진 텍스처 식별자가 풀 객체에 등록되어 있는 경우 풀 객체를 덮어씁니다.
    /// 
    #[must_use]
    pub fn spawn<'a, Id: Into<TextureID>>(
        device: &wgpu::Device, 
        texture_id: Id, 
        desc: &wgpu::TextureDescriptor<'a>
    ) -> Arc<TextureHandle> {
        // 텍스처 식별자를 생성합니다.
        let id = texture_id.into();

        // 텍스처를 생성합니다. (임계 영역 최소화)
        let texture: Arc<_> = TextureHandle::new(device, desc).into();
        
        {
            // 풀 객체의 lock을 획득합니다.
            let mut guard = POOL.lock().unwrap();

            // 풀 객체에 텍스처를 등록합니다.
            guard.insert(id, texture.clone());
        }

        return texture;
    }

    /// 텍스처를 생성 및 초기화 한 뒤, 주어진 텍스처 식별자로 풀 객체에 등록합니다.
    /// 
    /// 이미 주어진 텍스처 식별자가 풀 객체에 등록되어 있는 경우 풀 객체를 덮어씁니다.
    /// 
    #[must_use]
    pub fn spawn_with_data<'a, Id: Into<TextureID>>(
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        texture_id: Id, 
        desc: &wgpu::TextureDescriptor<'a>, 
        order: wgpu::util::TextureDataOrder, 
        data: &[u8]
    ) -> Arc<TextureHandle> {
        // 텍스처 식별자를 생성합니다.
        let id = texture_id.into();

        // 텍스처를 생성합니다. (임계 영역 최소화)
        let texture: Arc<_> = TextureHandle::new_with_data(device, queue, desc, order, data).into();

        {
            // 풀 객체의 lock을 획득합니다.
            let mut guard = POOL.lock().unwrap();

            // 풀 객체에 텍스처를 등록합니다.
            guard.insert(id, texture.clone());
        }

        return texture;
    }

    /// 주어진 텍스처 식별자의 텍스처를 풀 객체에서 가져옵니다.
    /// 풀 객체에 등록된 텍스처가 없는 경우 `None`을 반환합니다.
    pub fn get<Id: Into<TextureID>>(texture_id: Id) -> Option<Arc<TextureHandle>> {
        {
            // 풀 객체의 lock을 획득합니다.
            let guard = POOL.lock().unwrap();

            // 풀 객체에 등록된 텍스처를 가져옵니다.
            guard.get(&texture_id.into()).cloned()
        }
    }

    /// 주어진 텍스처 식별자의 텍스처를 풀 객체에서 제거합니다.
    pub fn remove<Id: Into<TextureID>>(texture_id: Id) {
        {
            // 풀 객체의 lock을 획득합니다.
            let mut guard = POOL.lock().unwrap();

            // 풀 객체에 등록된 텍스처를 제거합니다.
            guard.remove(&texture_id.into());
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
