//! 인게임 단계에서 총알 오브젝트 데이터 갱신과 관련된 코드를 관리합니다.
//!

use std::num::NonZeroU32;

use crate::components::{BigEndian, BulletKind, ObjectId, TryFromBigEndian};

/// 인게임 총알 오브젝트 갱신 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InGameBulletPullData {
    /// 오브젝트 식별자
    pub id: ObjectId,
    /// 총알의 종류
    pub kind: BulletKind,
    /// 총알의 월드 공간 위치
    translation: [i16; 3],
    /// 총알의 월드 공간 방향
    rotation: [i16; 4],
}

impl InGameBulletPullData {
    /// 새로운 `InGameBulletPullData`를 생성합니다.
    pub fn new(
        id: ObjectId,
        kind: BulletKind,
        half_size_x: NonZeroU32,
        half_size_y: NonZeroU32,
        half_size_z: NonZeroU32,
        translation: glam::Vec3A,
        rotation: glam::Quat,
    ) -> Self {
        let hx = half_size_x.get() as f32;
        let hy = half_size_y.get() as f32;
        let hz = half_size_z.get() as f32;
        let x = translation.x.clamp(-hx, hx) / hx * i16::MAX as f32;
        let y = translation.x.clamp(-hy, hy) / hy * i16::MAX as f32;
        let z = translation.x.clamp(-hz, hz) / hz * i16::MAX as f32;
        let translation = [x as i16, y as i16, z as i16];

        let rotation = rotation.normalize();
        let x = rotation.x * i16::MAX as f32;
        let y = rotation.y * i16::MAX as f32;
        let z = rotation.z * i16::MAX as f32;
        let w = rotation.w * i16::MAX as f32;
        let rotation = [x as i16, y as i16, z as i16, w as i16];

        Self {
            id,
            kind,
            translation,
            rotation,
        }
    }

    /// 플레이어의 월드 공간 위치를 반환합니다.
    pub fn translation(
        &self,
        half_size_x: NonZeroU32,
        half_size_y: NonZeroU32,
        half_size_z: NonZeroU32,
    ) -> glam::Vec3A {
        let x = self.translation[0] as f32 / i16::MAX as f32;
        let y = self.translation[1] as f32 / i16::MAX as f32;
        let z = self.translation[2] as f32 / i16::MAX as f32;
        let translation = glam::vec3a(x, y, z);
        let half_size = glam::vec3a(
            half_size_x.get() as f32,
            half_size_y.get() as f32,
            half_size_z.get() as f32,
        );
        translation * half_size
    }

    /// 플레이어의 월드 공간 방향을 반환합니다.
    pub fn rotation(&self) -> glam::Quat {
        let x = self.rotation[0] as f32 / i16::MAX as f32;
        let y = self.rotation[1] as f32 / i16::MAX as f32;
        let z = self.rotation[2] as f32 / i16::MAX as f32;
        let w = self.rotation[3] as f32 / i16::MAX as f32;
        let rotation = glam::quat(x, y, z, w);
        rotation.normalize()
    }
}

impl BigEndian for InGameBulletPullData {
    fn byte_size() -> usize {
        ObjectId::byte_size()
            + BulletKind::byte_size()
            + <[i16; 3]>::byte_size()
            + <[i16; 4]>::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());

        // 바이트 배열 유효성을 검증합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameBulletData)
            );
        }

        bytes
    }
}

impl TryFromBigEndian for InGameBulletPullData {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인합니다.
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(InGameBulletData)
            )
        };

        let mut offset = 0;
        let mut size = ObjectId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let id = ObjectId::from_big_endian_bytes(data);

        offset = offset + size;
        size = BulletKind::byte_size();
        data = &bytes[offset..offset + size];
        let kind = BulletKind::try_from_big_endian_bytes(data)?;

        offset = offset + size;
        size = <[i16; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let translation = <[i16; 3]>::from_big_endian_bytes(data);

        offset = offset + size;
        size = <[i16; 4]>::byte_size();
        data = &bytes[offset..offset + size];
        let rotation = <[i16; 4]>::from_big_endian_bytes(data);

        Some(Self {
            id,
            kind,
            translation,
            rotation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_create_bullet_kind() {
        let bytes = [127];
        BulletKind::from_big_endian_bytes(&bytes);
    }

    #[test]
    fn test_bullet_kind() {
        let origin = BulletKind::Common;
        let bytes = origin.to_big_endian_bytes();
        let other = BulletKind::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_bullet() {
        let origin = InGameBulletPullData::new(
            ObjectId::new(3141592),
            BulletKind::Common,
            NonZeroU32::new(25).unwrap(),
            NonZeroU32::new(25).unwrap(),
            NonZeroU32::new(25).unwrap(),
            glam::vec3a(-1.0101, 2.3456, 1000.011),
            glam::quat(0.1234, 1.99992, 0.08843, 1.0),
        );
        let bytes = origin.to_big_endian_bytes();
        let other = InGameBulletPullData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
