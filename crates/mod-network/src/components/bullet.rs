use super::{BigEndian, CharacterKind, ClientId, ObjectId, TryFromBigEndian};

/// 총알 모델 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BulletKind {
    Common = 0,
    ArisOriginal = 1,
}

impl BigEndian for BulletKind {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for BulletKind {
    fn default() -> Self {
        Self::Common
    }
}

impl From<CharacterKind> for BulletKind {
    fn from(value: CharacterKind) -> Self {
        match value {
            CharacterKind::ArisOriginal => BulletKind::ArisOriginal,
            CharacterKind::MomoiOriginal => BulletKind::Common,
            CharacterKind::MidoriOriginal => BulletKind::Common,
            CharacterKind::YuukaOriginal => BulletKind::Common,
        }
    }
}

impl TryFromBigEndian for BulletKind {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let index = u8::from_big_endian_bytes(bytes);
        match index {
            0 => Some(BulletKind::Common),
            1 => Some(BulletKind::ArisOriginal),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(BulletKind),
                    index
                );
                None
            }
        }
    }
}

/// 서버에서 클라이언트로 총알 정보를 보내는데 사용되는 구조체
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bullet {
    /// 총알의 오브젝트 식별자
    pub object_id: ObjectId,
    /// 총알을 발사한 클라이언트 식별자
    pub shooter_id: ClientId,
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

impl BigEndian for Bullet {
    fn byte_size() -> usize {
        ObjectId::byte_size()
            + ClientId::byte_size()
            + BulletKind::byte_size()
            + <[f32; 3]>::byte_size()
            + <[f32; 4]>::byte_size()
            + <[f32; 3]>::byte_size()
            + f32::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.object_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.shooter_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bullet_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.velocity.to_big_endian_bytes());
        bytes.extend_from_slice(&self.remaining_distance.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(Bullet)
            );
        }

        bytes
    }
}

impl Default for Bullet {
    fn default() -> Self {
        // object_id, shooter_id의 기본 값은 NULL이어야 합니다.
        Self {
            object_id: ObjectId::NULL,
            shooter_id: ClientId::NULL,
            bullet_kind: BulletKind::default(),
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            velocity: [0.0, 0.0, 0.0],
            remaining_distance: 0.0,
        }
    }
}

impl TryFromBigEndian for Bullet {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(Bullet)
        );

        // 오브젝트 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = ObjectId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let object_id = ObjectId::from_big_endian_bytes(data);

        // 클라이언트 식별자를 가져옵니다.
        offset = offset + size;
        size = ClientId::byte_size();
        data = &bytes[offset..offset + size];
        let shooter_id = ClientId::from_big_endian_bytes(data);

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
    fn creation_test_bullet_kind() {
        let bytes = [2];
        BulletKind::from_big_endian_bytes(&bytes);
    }

    #[test]
    fn validation_test_bullet_kind() {
        let origin = BulletKind::Common;
        let bytes = origin.to_big_endian_bytes();
        let other = BulletKind::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(BulletKind::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn validation_test_bullet() {
        let origin = Bullet {
            object_id: ObjectId::new(3141592),
            shooter_id: ClientId::new(577888),
            bullet_kind: BulletKind::Common,
            translation: [-1.0101, 2.3456, 1000.011],
            rotation: [0.1234, 1.99992, 0.08843, 1.0],
            velocity: [0.0, -0.1334, 0.5887],
            remaining_distance: 700.0,
            ..Default::default()
        };
        let bytes = origin.to_big_endian_bytes();
        let other = Bullet::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(Bullet::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
