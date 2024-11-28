use serde::{Deserialize, Serialize};

/// ## Two-Dimensional Vector Data
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Float2 {
    pub x: f32,
    pub y: f32,
}

impl Into<[f32; 2]> for Float2 {
    fn into(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

/// ## Three-Dimensional Vector Data
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Float3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Into<[f32; 3]> for Float3 {
    fn into(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

/// ## Four-Dimensional Vector Data
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Float4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Into<[f32; 4]> for Float4 {
    fn into(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

/// ## Four-Dimensional Vector Data (Unsigned Integer)
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Uint4 {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub w: u32,
}

impl Into<[u32; 4]> for Uint4 {
    fn into(self) -> [u32; 4] {
        [self.x, self.y, self.z, self.w]
    }
}

/// ## Matrix Data
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Matrix {
    pub m00: f32,
    pub m01: f32,
    pub m02: f32,
    pub m03: f32,
    pub m10: f32,
    pub m11: f32,
    pub m12: f32,
    pub m13: f32,
    pub m20: f32,
    pub m21: f32,
    pub m22: f32,
    pub m23: f32,
    pub m30: f32,
    pub m31: f32,
    pub m32: f32,
    pub m33: f32,
}

impl Into<[f32; 16]> for Matrix {
    fn into(self) -> [f32; 16] {
        [
            self.m00, self.m01, self.m02, self.m03, self.m10, self.m11, self.m12, self.m13,
            self.m20, self.m21, self.m22, self.m23, self.m30, self.m31, self.m32, self.m33,
        ]
    }
}

impl Into<glam::Mat4> for Matrix {
    fn into(self) -> glam::Mat4 {
        glam::mat4(
            glam::vec4(self.m00, self.m01, self.m02, self.m03),
            glam::vec4(self.m10, self.m11, self.m12, self.m13),
            glam::vec4(self.m20, self.m21, self.m22, self.m23),
            glam::vec4(self.m30, self.m31, self.m32, self.m33),
        )
    }
}

/// ## Texture View Dimension Data
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewDimension {
    D1,
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
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

/// ## Texture Address Mode Data
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressMode {
    ClampToEdge,
    Repeat,
    MirrorRepeat,
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

/// ## Texture Filtering Mode Data
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterMode {
    Nearest,
    Linear,
}

impl Into<wgpu::FilterMode> for FilterMode {
    fn into(self) -> wgpu::FilterMode {
        match self {
            FilterMode::Nearest => wgpu::FilterMode::Nearest,
            FilterMode::Linear => wgpu::FilterMode::Linear,
        }
    }
}

/// ## 3D Model Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelBlob {
    pub root: NodeBlob,
    pub animations: Vec<AnimationBlob>,
}

/// ## Node Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeBlob {
    pub name: String,
    pub transform: Matrix,
    pub mesh: Option<MeshBlob>,
    pub materials: Vec<MaterialBlob>,
    pub children: Vec<NodeBlob>,
}

/// ## Mesh Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeshBlob {
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
    pub skinning: Option<SkinningBlob>,
}

/// ## Material Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MaterialBlob {
    pub name: String,
    pub glossiness: Option<f32>,
    pub smoothness: Option<f32>,
    pub metallic: Option<f32>,
    pub bump_scale: Option<f32>,
    pub parallax: Option<f32>,
    pub strength: Option<f32>,
    pub albedo: Option<Float4>,
    pub specular: Option<Float4>,
    pub emissive: Option<Float4>,
    pub albedo_map: Option<TextureBlob>,
    pub specular_map: Option<TextureBlob>,
    pub emissive_map: Option<TextureBlob>,
    pub normal_map: Option<TextureBlob>,
    pub parallax_map: Option<TextureBlob>,
    pub occlusion_map: Option<TextureBlob>,
}

/// ## Texture View Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextureBlob {
    pub name: String, // Texture는 다른 파일에 저장됨.
    pub dimension: ViewDimension,
    pub address_u: AddressMode,
    pub address_v: AddressMode,
    pub address_w: AddressMode,
    pub filter_mode: FilterMode,
}

/// ## Skinning Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkinningBlob {
    pub quality: u32,
    pub root_bone: String,
    pub bones: Vec<String>,
    pub bindposes: Vec<Matrix>,
}

/// ## Animation Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationBlob {
    pub name: String,
    pub length: f32,
    pub frame_rate: f32,
    pub keyframes: Vec<KeyFrameBlob>,
}

/// ## Animation Key Frame Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameBlob {
    pub time_point: f32,
    pub root_matrix: Matrix,
    pub meshes: Vec<KeyFrameMeshBlob>,
}

/// ## Animation Key Frame Skinned Mesh Data Blob
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameMeshBlob {
    pub name: String,
    pub bone_trans: Vec<Matrix>,
}
