/// 총알 오브젝트
/// 
/// 데이터
/// 1. 총알 종류
/// 
/// 2. 발사한 유저의 식별자
/// 
/// 3. 위치
/// 
/// 4. 회전
/// 
/// 5. 속력
/// 
/// 6. 사거리
/// 
/// 7. 충돌체
/// 
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bullet {
    pub kind: u32, 
    pub shooter: u32, 
    pub translation: gmm::Float3, 
    pub rotation: gmm::Float4, 
    pub speed: f32, 
    pub range: f32, 
    // TODO: 충돌체 추가
}

impl Bullet {
    #[inline]
    #[must_use]
    pub fn new(
        kind: u32, 
        shooter: u32, 
        translation: impl Into<gmm::Float3>, 
        rotation: impl Into<gmm::Float4>, 
        speed: f32, 
        range: f32, 
        // TODO: 충돌체 추가
    ) -> Self {
        Self { 
            kind, 
            shooter, 
            translation: translation.into(), 
            rotation: rotation.into(), 
            speed, 
            range, 
            // TODO: 충돌체 추가
        }
    }


    /// `big-endian` 바이트 배열로부터 `Bullet`을 생성합니다.
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Self {
        Self::new(
            u32::from_be_bytes(data[0..4].try_into().unwrap()), 
            u32::from_be_bytes(data[4..8].try_into().unwrap()), 
            gmm::Float3::new(
                f32::from_be_bytes(data[8..12].try_into().unwrap()), 
                f32::from_be_bytes(data[12..16].try_into().unwrap()), 
                f32::from_be_bytes(data[16..20].try_into().unwrap())
            ), 
            gmm::Float4::new(
                f32::from_be_bytes(data[20..24].try_into().unwrap()), 
                f32::from_be_bytes(data[24..28].try_into().unwrap()), 
                f32::from_be_bytes(data[28..32].try_into().unwrap()), 
                f32::from_be_bytes(data[32..36].try_into().unwrap())
            ), 
            f32::from_be_bytes(data[36..40].try_into().unwrap()), 
            f32::from_le_bytes(data[40..44].try_into().unwrap())
        )
    }


    /// `big-endian` 바이트 배열을 반환합니다.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(std::mem::size_of::<Self>());
        bytes.extend_from_slice(&self.kind.to_be_bytes());
        bytes.extend_from_slice(&self.shooter.to_be_bytes());
        bytes.extend_from_slice(&self.translation.x.to_be_bytes());
        bytes.extend_from_slice(&self.translation.y.to_be_bytes());
        bytes.extend_from_slice(&self.translation.z.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.x.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.y.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.z.to_be_bytes());
        bytes.extend_from_slice(&self.rotation.w.to_be_bytes());
        bytes.extend_from_slice(&self.speed.to_be_bytes());
        bytes.extend_from_slice(&self.range.to_be_bytes());
        // TODO: 충돌체를 big-endian 바이트 배열로 변환
        bytes
    }
}
