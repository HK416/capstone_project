mod hierarchy;
mod motion;

use std::io;

use serde::{Deserialize, Serialize};

pub use self::{hierarchy::*, motion::*};

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

impl Matrix {
    pub fn into_mat4(&self) -> glam::Mat4 {
        glam::mat4(
            glam::vec4(self.m00, self.m01, self.m02, self.m03),
            glam::vec4(self.m10, self.m11, self.m12, self.m13),
            glam::vec4(self.m20, self.m21, self.m22, self.m23),
            glam::vec4(self.m30, self.m31, self.m32, self.m33),
        )
    }
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

/// ## Model Load Error List
#[derive(Debug, thiserror::Error)]
pub enum ModelAssetError {
    /// dds 포맷의 텍스처를 읽는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to read texture for the following reason:{0}")]
    TextureError(#[from] ddsfile::Error),

    /// 에셋 파일을 구문 분석하는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to parse asset for the following reason:{1} (PATH:{0})")]
    ParsingFailed(String, serde_json::Error),

    /// 파일을 열거나 읽을 때 발생하는 오류입니다.
    #[error("failed to read asset for the following reason:{1} (PATH:{0})")]
    IOError(String, io::Error),
}
