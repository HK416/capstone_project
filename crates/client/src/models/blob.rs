use serde::{Deserialize, Serialize};

/// ## Two-Dimensional Vector Data
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Float2 {
    pub x: f32, pub y: f32
}

/// ## Three-Dimensional Vector Data
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Float3 {
    pub x: f32, pub y: f32, pub z: f32
}

/// ## Four-Dimensional Vector Data
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Float4 {
    pub x: f32, pub y: f32, pub z: f32, pub w: f32
}

/// ## Four-Dimensional Vector Data (Unsigned Integer)
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Uint4 {
    pub x: u32, pub y: u32, pub z: u32, pub w: u32
}

/// ## Texture View Dimension Data
#[derive(Deserialize, Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewDimension {
    D1, 
    D2, 
    D2Array, 
    Cube, 
    CubeArray, 
    D3
}

/// ## Texture Address Mode Data
#[derive(Deserialize, Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressMode {
    ClampToEdge, 
    Repeat, 
    MirrorRepeat
}

/// ## Texture Filtering Mode Data
#[derive(Deserialize, Serialize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterMode {
    Nearest, 
    Linear
}

/// ## Matrix Data
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Matrix {
    pub m00: f32, pub m01: f32, pub m02: f32, pub m03: f32, 
    pub m10: f32, pub m11: f32, pub m12: f32, pub m13: f32, 
    pub m20: f32, pub m21: f32, pub m22: f32, pub m23: f32, 
    pub m30: f32, pub m31: f32, pub m32: f32, pub m33: f32, 
}

/// ## 3D Model Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelBlob {
    pub root: NodeBlob, 
    pub animations: Vec<AnimationBlob>
}

/// ## Node Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeBlob {
    pub name: String, 
    pub transform: Matrix, 
    pub mesh: Option<MeshBlob>, 
    pub children: Vec<NodeBlob> 
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
    pub materials: Vec<MaterialBlob>, 
    pub skinning: Option<SkinningBlob>, 
}

/// ## Material Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MaterialBlob {
    pub name: String, 
    pub glossiness: Option<f32>, 
    pub smoothness: Option<f32>, 
    pub metallic: Option<f32>, 
    pub height: Option<f32>, 
    pub albedo: Option<Float4>, 
    pub specular: Option<Float4>, 
    pub emissive: Option<Float4>, 
    pub albedo_map: Option<TextureBlob>, 
    pub specular_map: Option<TextureBlob>, 
    pub emissive_map: Option<TextureBlob>, 
    pub normal_map: Option<TextureBlob>, 
    pub height_map: Option<TextureBlob>
}

/// ## Texture Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextureBlob {
    pub name: String, 
    pub dimension: ViewDimension, 
    pub address_u: AddressMode, 
    pub address_v: AddressMode, 
    pub address_w: AddressMode, 
    pub filter_mode: FilterMode 
}

/// ## Skinning Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkinningBlob {
    pub quality: u32, 
    pub root_bone: String, 
    pub bones: Vec<String>, 
    pub bindposes: Vec<Matrix>
}

/// ## Animation Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimationBlob {
    pub name: String, 
    pub root_bone: String, 
    pub length: f32, 
    pub frame_rate: f32, 
    pub keyframes: Vec<KeyFrameBlob> 
}

/// ## Animation Key Frame Data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameBlob {
    pub time_point: f32, 
    pub root_matrix: Matrix, 
    pub meshes: Vec<KeyFrameMeshBlob>
}

/// ## Animation Key Frame Skinned Mesh Data Blob
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyFrameMeshBlob {
    pub name: String, 
    pub bone_trans: Vec<Matrix>
}
