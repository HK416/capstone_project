mod core;
mod game;
mod input;
mod rendering;
mod system;

#[allow(ambiguous_glob_reexports)]
pub use self::{core::*, game::*, input::*, rendering::*, system::*};

use serde::{Deserialize, Serialize};

/// 자료형을 Big-endian 바이트 배열로 변환하거나, Big-endian 바이트 배열로부터 자료형을 생성하는 함수 인터페이스를 제공합니다.
pub trait BigEndian {
    /// Big-endian 바이트 배열의 크기를 반환합니다.
    fn byte_size() -> usize
    where
        Self: Sized,
    {
        std::mem::size_of::<Self>()
    }

    /// Big-endian 바이트 배열로부터 자료형을 생성합니다.
    ///
    /// # Panics
    /// 바이트 배열의 크기가 자료형의 크기와 다른 경우 [`panic!`]을 호출합니다.
    ///
    fn from_big_endian_bytes(bytes: &[u8]) -> Self
    where
        Self: Sized;

    /// 자료형을 Big-endian 바이트 배열로 변환합니다.
    fn to_big_endian_bytes(&self) -> Vec<u8>;
}

impl BigEndian for i8 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for u8 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for i16 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for u16 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for i32 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for u32 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for f32 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for i64 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for u64 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for f64 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for i128 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for u128 {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes(bytes.try_into().unwrap())
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl BigEndian for [f32; 3] {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        [
            f32::from_big_endian_bytes(&bytes[0..4]),
            f32::from_big_endian_bytes(&bytes[4..8]),
            f32::from_big_endian_bytes(&bytes[8..12]),
        ]
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12);
        bytes.extend_from_slice(&self[0].to_big_endian_bytes());
        bytes.extend_from_slice(&self[1].to_big_endian_bytes());
        bytes.extend_from_slice(&self[2].to_big_endian_bytes());
        bytes
    }
}

impl BigEndian for [f32; 4] {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        [
            f32::from_big_endian_bytes(&bytes[0..4]),
            f32::from_big_endian_bytes(&bytes[4..8]),
            f32::from_big_endian_bytes(&bytes[8..12]),
            f32::from_big_endian_bytes(&bytes[12..16]),
        ]
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&self[0].to_big_endian_bytes());
        bytes.extend_from_slice(&self[1].to_big_endian_bytes());
        bytes.extend_from_slice(&self[2].to_big_endian_bytes());
        bytes.extend_from_slice(&self[3].to_big_endian_bytes());
        bytes
    }
}

/// 자료형을 Big-endian 바이트 배열로 변환하거나, Big-endian 바이트 배열로부터 자료형을 생성하는 함수 인터페이스를 제공합니다.
pub trait TryFromBigEndian: BigEndian {
    /// Big-endian 바이트 배열로부터 자료형을 생성합니다.
    /// 자료형 생성에 실패한 경우 `None`을 반환합니다.
    ///
    /// # Panics
    /// 바이트 배열의 크기가 자료형의 크기와 다른 경우 [`panic!`]을 호출합니다.
    ///
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self>
    where
        Self: Sized;
}

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
