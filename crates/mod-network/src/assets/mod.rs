pub mod model;
pub mod stage;

use serde::{Deserialize, Serialize};

pub use self::{model::*, stage::*};

/// 2차원 실수형 벡터
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

impl Into<glam::Vec2> for Float2 {
    fn into(self) -> glam::Vec2 {
        glam::vec2(self.x, self.y)
    }
}

/// 3차원 실수형 벡터
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Float3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Into<(f32, f32, f32)> for Float3 {
    fn into(self) -> (f32, f32, f32) {
        (self.x, self.y, self.z)
    }
}

impl Into<[f32; 3]> for Float3 {
    fn into(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl Into<glam::Vec3> for Float3 {
    fn into(self) -> glam::Vec3 {
        glam::vec3(self.x, self.y, self.z)
    }
}

/// 4차원 실수형 벡터
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

impl Into<glam::Vec4> for Float4 {
    fn into(self) -> glam::Vec4 {
        glam::vec4(self.x, self.y, self.z, self.w)
    }
}

impl Into<glam::Quat> for Float4 {
    fn into(self) -> glam::Quat {
        glam::quat(self.x, self.y, self.z, self.w)
    }
}

/// 4차원 정수형 벡터
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

/// 4x4 실수형 행렬
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
