use serde::{Deserialize, Serialize};

use crate::components::{Float2, Float3, Float4, Matrix, Uint4};

/// 모델의 계층 구조 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelHierarchyData {
    pub root: HierarchyNode,
    pub num_nodes: u32,
}

/// 모델의 계층 구조를 구성하는 노드 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HierarchyNode {
    pub name: String,
    pub transform: Matrix,
    pub mesh: Option<String>,
    pub materials: Vec<String>,
    pub children: Vec<HierarchyNode>,
}

/// 모델 메쉬 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeshData {
    pub name: String,
    pub minimum: Float3,
    pub maximum: Float3,
    pub vertices: Vec<Float3>,
    pub colors: Vec<Float4>,
    pub normals: Vec<Float3>,
    pub tangents: Vec<Float3>,
    pub texcoords0: Vec<Float2>,
    pub texcoords1: Vec<Float2>,
    pub texcoords2: Vec<Float2>,
    pub texcoords3: Vec<Float2>,
    pub bone_indices: Vec<Uint4>,
    pub bone_weights: Vec<Float4>,
    pub submeshes: Vec<Vec<u32>>,
    pub skinning: Option<SkinningData>,
}

/// 모델 메쉬의 스키닝 애니메이션 데이터입니다.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkinningData {
    pub quality: u32,
    pub root_bone: String,
    pub bones: Vec<String>,
    pub bindposes: Vec<Matrix>,
}

/// 재질 텍스처 데이터
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextureData {
    pub name: String,
    pub dimension: ViewDimension,
    pub address_u: AddressMode,
    pub address_v: AddressMode,
    pub address_w: AddressMode,
    pub filter_mode: FilterMode,
}

/// 텍스처 뷰 차원
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum ViewDimension {
    D1 = 0,
    D2 = 1,
    D2Array = 2,
    Cube = 3,
    CubeArray = 4,
    D3 = 5,
}

impl Into<wgpu::TextureViewDimension> for ViewDimension {
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

/// 텍스처 샘플러 Address
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum AddressMode {
    ClampToEdge = 0,
    Repeat = 1,
    MirrorRepeat = 2,
}

impl Into<wgpu::AddressMode> for AddressMode {
    fn into(self) -> wgpu::AddressMode {
        match self {
            AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            AddressMode::Repeat => wgpu::AddressMode::Repeat,
            AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        }
    }
}

/// 텍스처 샘플러 필터링 모드
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum FilterMode {
    Nearest = 0,
    Linear = 1,
}

impl Into<wgpu::FilterMode> for FilterMode {
    fn into(self) -> wgpu::FilterMode {
        match self {
            FilterMode::Nearest => wgpu::FilterMode::Nearest,
            FilterMode::Linear => wgpu::FilterMode::Linear,
        }
    }
}
