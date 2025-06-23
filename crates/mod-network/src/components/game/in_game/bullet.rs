//! 인게임 단계에서 총알 오브젝트 데이터 갱신과 관련된 코드를 관리합니다.
//!

use crate::components::{BigEndian, BulletKind, ObjectId, TryFromBigEndian, UserId};

/// 인게임 총알 오브젝트 갱신 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InGameBulletPullData {
    /// 총알의 오브젝트 식별자
    pub object_id: ObjectId,
    /// 총알을 발사한 클라이언트 식별자
    pub shooter_id: UserId,
    /// 총알의 종류
    pub bullet_kind: BulletKind,
    /// 총알의 월드 공간 위치
    pub translation: [f32; 3],
    /// 총알의 월드 공간 방향
    pub rotation: [f32; 4],
    /// 총알의 월드 공간 속도
    pub velocity: [f32; 3],
    /// 총알의 남은 사거리
    pub remaining_distance: f32,
}

impl BigEndian for InGameBulletPullData {
    fn byte_size() -> usize {
        ObjectId::byte_size()    // 4byte
            + UserId::byte_size()    // 8byte
            + BulletKind::byte_size()    // 9byte
            + <[f32; 3]>::byte_size()    // 21byte
            + <[f32; 4]>::byte_size()    // 37byte
            + <[f32; 3]>::byte_size()    // 49byte
            + f32::byte_size() // 53byte
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.object_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.shooter_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bullet_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.velocity.to_big_endian_bytes());
        bytes.extend_from_slice(&self.remaining_distance.to_big_endian_bytes());

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

        // 오브젝트 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = ObjectId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let object_id = ObjectId::from_big_endian_bytes(data);

        // 클라이언트 식별자를 가져옵니다.
        offset = offset + size;
        size = UserId::byte_size();
        data = &bytes[offset..offset + size];
        let shooter_id = UserId::from_big_endian_bytes(data);

        // 총알 종류를 가져옵니다.
        offset = offset + size;
        size = BulletKind::byte_size();
        data = &bytes[offset..offset + size];
        let bullet_kind = BulletKind::try_from_big_endian_bytes(data)?;

        // 위치를 가져옵니다.
        offset = offset + size;
        size = <[f32; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let translation = <[f32; 3]>::from_big_endian_bytes(data);

        // 방향을 가져옵니다.
        offset = offset + size;
        size = <[f32; 4]>::byte_size();
        data = &bytes[offset..offset + size];
        let rotation = <[f32; 4]>::from_big_endian_bytes(data);

        // 속도를 가져옵니다.
        offset = offset + size;
        size = <[f32; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let velocity = <[f32; 3]>::from_big_endian_bytes(data);

        // 남은 거리를 가져옵니다.
        offset = offset + size;
        size = f32::byte_size();
        data = &bytes[offset..offset + size];
        let remaining_distance = f32::from_big_endian_bytes(data);

        Some(Self {
            object_id,
            shooter_id,
            bullet_kind,
            translation,
            rotation,
            velocity,
            remaining_distance,
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
        let origin = InGameBulletPullData {
            object_id: ObjectId::new(3141592),
            shooter_id: UserId::new(577888),
            bullet_kind: BulletKind::Common,
            translation: [-1.0101, 2.3456, 1000.011],
            rotation: [0.1234, 1.99992, 0.08843, 1.0],
            velocity: [0.0, -0.1334, 0.5887],
            remaining_distance: 700.0,
        };
        let bytes = origin.to_big_endian_bytes();
        let other = InGameBulletPullData::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인합니다.
        assert_eq!(origin, other);
    }
}
