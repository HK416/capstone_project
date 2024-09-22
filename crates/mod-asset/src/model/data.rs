use mod_physics::BoundingBox;
use serde::{Deserialize, Serialize};



/// 모델 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelBlob {
    /// 모델을 구성하는 노드 데이터입니다.
    pub root: NodeBlob, 

    /// 모델에 포함된 애니메이션 데이터입니다.
    pub animations: Vec<AnimationBlob>, 
}



/// 모델을 구성하는 노드 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeBlob {
    /// 노드의 이름입니다.
    pub name: String, 

    /// 노드의 부모로 부터 변환 행렬입니다.
    pub transform: gmm::Float4x4, 

    /// 노드에 연결된 메쉬 데이터입니다.
    pub mesh: Option<MeshBlob>, 

    /// 노드에 연결된 스키닝 데이터입니다.
    pub skin: Option<SkinBlob>, 

    /// 노드에 연결된 재질 데이터입니다.
    pub materials: Vec<MaterialBlob>, 

    /// 노드에 연결된 하위 노드 데이터입니다.
    pub children: Vec<NodeBlob>, 
}



/// 노드에 연결된 메쉬 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeshBlob {
    /// 메쉬의 이름입니다.
    pub name: String, 

    /// 메쉬의 정점 데이터입니다.
    pub vertices: Vec<gmm::Float3>, 

    /// 메쉬 정점의 색상 데이터입니다.
    pub colors: Vec<gmm::Float4>, 

    /// 메쉬 정점의 노멀 데이터입니다.
    pub normals: Vec<gmm::Float3>, 

    /// 메쉬 정점의 탄젠트 공간 노멀 데이터입니다.
    pub tangents: Vec<gmm::Float3>, 

    /// 메쉬 정점의 0번 텍스처 좌표 데이터입니다.
    pub texcoords0: Vec<gmm::Float2>, 

    /// 메쉬 정점의 1번 텍스처 좌표 데이터입니다.
    pub texcoords1: Vec<gmm::Float2>, 

    /// 메쉬 정점의 뼈 번호 데이터입니다.
    pub bone_indices: Vec<gmm::UInteger4>, 

    /// 메쉬 정점의 뼈 가중치 데이터입니다.
    pub bone_weights: Vec<gmm::Float4>, 

    /// 메쉬의 하위 메쉬 데이터입니다.
    pub submeshes: Vec<Vec<u32>>, 

    /// 메쉬의 축 정렬 경계 상자 데이터입니다.
    pub bounds: BoundingBox, 
}



/// 노드에 연결된 스키닝 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkinBlob {
    /// 정점에 연결된 뼈의 개수입니다. (최대 4개)
    pub quality: u32, 

    /// 최상위 뼈 노드의 이름입니다.
    pub root_bone: String, 

    /// 스키닝 데이터를 구성하는 뼈 노드들의 이름입니다.
    pub bone_names: Vec<String>, 

    /// 바인드 포즈 변환 행렬 데이터입니다.
    pub bindposes: Vec<gmm::Float4x4>, 
}



/// 노드에 연결된 재질 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MaterialBlob {
    /// 재질의 이름입니다.
    pub name: String, 

    /// 재질의 매끄러운 정도입니다.
    pub glossiness: Option<f32>, 

    /// 재질의 부드러운 정도입니다.
    pub smoothness: Option<f32>, 

    /// 재질의 금속성 정도입니다.
    pub metallic: Option<f32>, 

    /// 재질의 `Diffuse` 색상입니다.
    pub diffuse: Option<gmm::Float4>, 

    /// 재질의 `Specular` 색상입니다.
    pub specular: Option<gmm::Float4>, 

    /// 재질의 `Emissive` 색상입니다.
    pub emissive: Option<gmm::Float4>, 

    /// 재질의 `Diffuse` 텍스처 데이터입니다.
    pub diffuse_map: Option<TextureBlob>, 

    /// 재질의 `Specular` 텍스처 데이터입니다.
    pub specular_map: Option<TextureBlob>, 

    /// 재질의 `Normal` 텍스처 데이터입니다.
    pub normal_map: Option<TextureBlob>, 

    /// 재질의 `Emissive` 텍스처 데이터입니다.
    pub emissive_map: Option<TextureBlob>, 
}



/// 재질에 연결된 텍스처 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextureBlob {
    /// 텍스처 이름입니다.
    pub name: String, 

    /// 텍스처의 차원입니다.
    pub dimension: ViewDimension, 

    /// 텍스처 샘플러의 필터링 모드입니다.
    pub filter_mode: FilterMode, 

    /// 텍스처 샘플러의 u 좌표계 매핑 모드입니다.
    pub address_u: AddressMode, 

    /// 텍스처 샘플러의 v 좌표계 매핑 모드입니다.
    pub address_v: AddressMode, 

    /// 텍스처 샘플러의 w 좌표계 매핑 모드입니다.
    pub address_w: AddressMode, 
}



/// 텍스처 차원 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ViewDimension {
    D1, 
    D2, 
    D2Array, 
    Cube, 
    CubeArray, 
    D3
}

impl Into<wgpu::TextureViewDimension> for ViewDimension {
    #[inline]
    fn into(self) -> wgpu::TextureViewDimension {
        match self {
            ViewDimension::D1 => wgpu::TextureViewDimension::D1,
            ViewDimension::D2 => wgpu::TextureViewDimension::D2,
            ViewDimension::D2Array => wgpu::TextureViewDimension::D2Array,
            ViewDimension::Cube => wgpu::TextureViewDimension::Cube,
            ViewDimension::CubeArray => wgpu::TextureViewDimension::CubeArray,
            ViewDimension::D3 => wgpu::TextureViewDimension::D3,
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



/// 모델에 연결된 애니메이션 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationBlob {
    /// 애니메이션의 이름입니다.
    pub name: String, 

    /// 애니메이션의 총 길이입니다.
    pub length: f32, 

    /// 애니메이션 샘플링 프레임 레이트입니다.
    pub frame_rate: f32, 

    /// 애니메이션 키 프레임 데이터입니다.
    pub keyframes: Vec<KeyFrameBlob>, 
}



/// 애니메이션을 구성하는 키 프레임 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameBlob {
    /// 키 프레임의 시각 데이터입니다.
    pub time_point: f32, 

    /// 키 프레임에 영향을 받는 스키닝 데이터입니다.
    pub meshes: Vec<KeyFrameMeshBlob>, 
}



/// 키 프레임에 연결된 스키닝 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameMeshBlob {
    /// 스키닝된 메쉬의 이름입니다.
    pub mesh_name: String, 

    /// 현재 키 프레임에 뼈 노드의 부모로 부터 변환 행렬입니다.
    pub bone_transforms: Vec<BoneTransformBlob>, 
}



/// 뼈 변환 데이터 노드입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoneTransformBlob {
    pub scale: gmm::Float3, 
    pub rotation: gmm::Float4, 
    pub translation: gmm::Float3, 
}
