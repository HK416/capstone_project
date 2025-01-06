use crate::components::BigEndian;

use super::super::components::ObjectId;


/// 총알 오브젝트
/// 
/// 데이터
/// 1. 총알 종류
/// 
/// 2. 발사한 유저의 식별자
/// 
/// 3. 위치
/// 
/// 4. 방향
/// 
/// 5. 속력
/// 
/// 6. 사거리
/// 
/// 7. 충돌체
/// 
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BulletBlob {
    pub kind: u32, 
    pub shooter: ObjectId, 
    pub translation: gmm::Float3, 
    pub direction: gmm::Float3, 
    pub speed: f32, 
    pub range: f32, 
    pub id: ObjectId,
    // TODO: 충돌체 추가
}

impl BulletBlob {
    #[inline]
    #[must_use]
    pub fn new(
        kind: u32, 
        shooter: ObjectId, 
        translation: impl Into<gmm::Float3>, 
        direction: impl Into<gmm::Float3>, 
        speed: f32, 
        range: f32, 
        // TODO: 충돌체 추가
    ) -> Self {
        Self { 
            kind, 
            shooter, 
            translation: translation.into(), 
            direction: direction.into(), 
            speed, 
            range, 
            id: ObjectId::new(0),      // 클라이언트에서 서버로 보낼때는 일단 아무 값이나 넣어서 보냄
            // TODO: 충돌체 추가
        }
    }

    pub fn with_id(
        kind: u32, 
        shooter: ObjectId, 
        translation: impl Into<gmm::Float3>, 
        direction: impl Into<gmm::Float3>, 
        speed: f32, 
        range: f32, 
        id: ObjectId,
    ) -> Self {
        Self { 
            kind, 
            shooter, 
            translation: translation.into(), 
            direction: direction.into(), 
            speed, 
            range, 
            id, 
        }
    }


    /// `big-endian` 바이트 배열로부터 `Bullet`을 생성합니다.
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Self {
        Self::with_id(
            u32::from_be_bytes(data[0..4].try_into().unwrap()), 
            ObjectId::from_big_endian_bytes(&data[4..8]), 
            gmm::Float3::new(
                f32::from_be_bytes(data[8..12].try_into().unwrap()), 
                f32::from_be_bytes(data[12..16].try_into().unwrap()), 
                f32::from_be_bytes(data[16..20].try_into().unwrap())
            ), 
            gmm::Float3::new(
                f32::from_be_bytes(data[20..24].try_into().unwrap()), 
                f32::from_be_bytes(data[24..28].try_into().unwrap()), 
                f32::from_be_bytes(data[28..32].try_into().unwrap()), 
            ), 
            f32::from_be_bytes(data[32..36].try_into().unwrap()), 
            f32::from_be_bytes(data[36..40].try_into().unwrap()),
            ObjectId::from_big_endian_bytes(&data[40..44]),
        )
    }


    /// `big-endian` 바이트 배열을 반환합니다.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(std::mem::size_of::<Self>());
        bytes.extend_from_slice(&self.kind.to_be_bytes());
        bytes.extend_from_slice(&self.shooter.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.x.to_be_bytes());
        bytes.extend_from_slice(&self.translation.y.to_be_bytes());
        bytes.extend_from_slice(&self.translation.z.to_be_bytes());
        bytes.extend_from_slice(&self.direction.x.to_be_bytes());
        bytes.extend_from_slice(&self.direction.y.to_be_bytes());
        bytes.extend_from_slice(&self.direction.z.to_be_bytes());
        bytes.extend_from_slice(&self.speed.to_be_bytes());
        bytes.extend_from_slice(&self.range.to_be_bytes());
        bytes.extend_from_slice(&self.id.to_big_endian_bytes());
        // TODO: 충돌체를 big-endian 바이트 배열로 변환
        bytes
    }
}
