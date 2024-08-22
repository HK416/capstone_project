use mod_physics::BoundingBox;
use serde::Deserialize;
use serde::Serialize;



/// 최상위 모델 데이터 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RootModelNode {
    pub root: ModelNode, 
    pub animations: Vec<AnimationNode>, 
}



/// 모델 데이터 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelNode {
    pub name: String, 
    pub transform: gmm::Float4x4, 
    pub mesh: Option<MeshNode>, 
    pub skin: Option<SkinNode>, 
    pub materials: Vec<MaterialNode>, 
    pub children: Vec<ModelNode>, 
}



/// 모델에 연결되어있는 메쉬 데이터 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeshNode {
    pub name: String, 
    pub vertices: Vec<gmm::Float3>, 
    pub colors: Vec<gmm::Float4>, 
    pub normals: Vec<gmm::Float3>, 
    pub tangents: Vec<gmm::Float3>, 
    pub texcoords0: Vec<gmm::Float2>, 
    pub texcoords1: Vec<gmm::Float2>, 
    pub bone_indices: Vec<gmm::UInteger4>, 
    pub bone_weights: Vec<gmm::Float4>, 
    pub bindposes: Vec<gmm::Float4x4>, 
    pub submeshes: Vec<Vec<u32>>, 
    pub bounds: BoundingBox, 
}



/// 모델에 연결되어있는 스키닝 데이터 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkinNode {
    pub quality: u32, 
    pub root_bone: String, 
    pub bone_names: Vec<String>, 
}



/// 모델에 연결되어있는 재질 데이터 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MaterialNode {
    pub name: String, 
    pub glossiness: Option<f32>, 
    pub smoothness: Option<f32>, 
    pub metallic: Option<f32>, 
    pub diffuse: Option<gmm::Float4>, 
    pub specular: Option<gmm::Float4>, 
    pub emissive: Option<gmm::Float4>, 
    pub diffuse_map: Option<TextureNode>, 
    pub specular_map: Option<TextureNode>, 
    pub normal_map: Option<TextureNode>, 
    pub emissive_map: Option<TextureNode>, 
}



/// 재질에 연결된 텍스처 데이터 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextureNode {
    pub name: String, 
    pub view_dimension: ViewDimension, 
    pub filter_mode: FilterMode, 
    pub address_u: AddressMode, 
    pub address_v: AddressMode, 
    pub address_w: AddressMode, 
}

/// 텍스처 뷰의 차원입니다.
#[derive(Deserialize, Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ViewDimension {
    Auto, 
    D1, 
    D2, 
    D2Array, 
    Cube, 
    CubeArray, 
    D3, 
}

impl Into<Option<wgpu::TextureViewDimension>> for ViewDimension {
    #[inline]
    fn into(self) -> Option<wgpu::TextureViewDimension> {
        match self {
            ViewDimension::Auto => None, 
            ViewDimension::D1 => Some(wgpu::TextureViewDimension::D1), 
            ViewDimension::D2 => Some(wgpu::TextureViewDimension::D2), 
            ViewDimension::D2Array => Some(wgpu::TextureViewDimension::D2Array), 
            ViewDimension::Cube => Some(wgpu::TextureViewDimension::Cube), 
            ViewDimension::CubeArray => Some(wgpu::TextureViewDimension::CubeArray), 
            ViewDimension::D3 => Some(wgpu::TextureViewDimension::D3), 
        }
    }
}

/// 텍스처 샘플러의 필터 모드입니다.
#[derive(Deserialize, Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FilterMode {
    Nearest, 
    Linear, 
}

impl Into<wgpu::FilterMode> for FilterMode {
    #[inline]
    fn into(self) -> wgpu::FilterMode {
        match self {
            FilterMode::Nearest => wgpu::FilterMode::Nearest, 
            FilterMode::Linear => wgpu::FilterMode::Linear, 
        }
    }
}

/// 텍스터 샘플러의 좌표 모드입니다.
#[derive(Deserialize, Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddressMode {
    ClampToEdge, 
    Repeat, 
    MirrorRepeat, 
}

impl Into<wgpu::AddressMode> for AddressMode {
    #[inline]
    fn into(self) -> wgpu::AddressMode {
        match self {
            AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge, 
            AddressMode::Repeat => wgpu::AddressMode::Repeat, 
            AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat, 
        }
    }
}



/// 모델의 애니메이션 데이터 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationNode {
    pub name: String, 
    pub length: f32, 
    pub frame_rate: f32, 
    pub keyframes: Vec<KeyFrameNode>, 
}



/// 애니메이션의 키 프레임 데이터 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameNode {
    pub time_point: f32, 
    pub meshes: Vec<KeyFrameMeshNode>, 
}



/// 키 프레임에 연결된 스키닝 메쉬 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameMeshNode {
    pub mesh_name: String, 
    pub bone_transforms: Vec<BoneTransformNode>, 
}



/// 뼈 변환 데이터 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoneTransformNode {
    pub scale: gmm::Float3, 
    pub rotation: gmm::Float4, 
    pub translation: gmm::Float3, 
}
