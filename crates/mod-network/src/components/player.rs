use super::{
    ActionState, ActionStateTimer, BigEndian, CharacterKind, HealthPoint, LatLon, MovementState,
    MovementStateTimer, ObjectId, TryFromBigEndian, ViewState, ViewStateTimer,
};

/// 서버에서 클라이언트로 플레이어 캐릭터 데이터를 보내는데 사용되는 구조체
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Player {
    /// 플레이어 캐릭터 오브젝트 식별자
    pub object_id: ObjectId,
    /// 플레이어 캐릭터의 종류
    pub character_kind: CharacterKind,
    /// 플레이어 캐릭터 체력
    pub health_point: HealthPoint,
    /// 플레이어 캐릭터의 월드 공간 위치
    pub translation: [f32; 3],
    /// 플레이어 캐릭터가 바라보는 월드 공간 방향 (캐릭터가 움직이는 방향과 다를 수 있음)
    pub rotation: [f32; 4],
    /// 플레이어 캐릭터의 월드 공간 속도
    pub velocity: [f32; 3],
    /// 플레이어 캐릭터의 행동 상태
    pub action_state: ActionState,
    /// 플레이어 캐릭터의 행동 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 플레이어 캐릭터의 움직임 상태
    pub movement_state: MovementState,
    /// 플레이어 캐릭터의 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
    /// 플레이어 카메라 상태
    pub view_state: ViewState,
    /// 플레이어 카메라 상태 타이머
    pub view_state_timer: ViewStateTimer,
    /// 플레이어 카메라가 캐릭터를 중심으로 바라보는 방향
    pub view_rotation: LatLon,
}

impl BigEndian for Player {
    fn byte_size() -> usize {
        ObjectId::byte_size()
            + CharacterKind::byte_size()
            + HealthPoint::byte_size()
            + <[f32; 3]>::byte_size()
            + <[f32; 4]>::byte_size()
            + <[f32; 3]>::byte_size()
            + ActionState::byte_size()
            + ActionStateTimer::byte_size()
            + MovementState::byte_size()
            + MovementStateTimer::byte_size()
            + ViewState::byte_size()
            + ViewStateTimer::byte_size()
            + LatLon::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.object_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.health_point.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.velocity.to_big_endian_bytes());
        bytes.extend_from_slice(&self.action_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.action_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.movement_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.movement_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.view_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.view_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.view_rotation.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(Player)
            );
        }

        bytes
    }
}

impl Default for Player {
    fn default() -> Self {
        // object_id의 기본 값은 NULL이어야 합니다.
        Self {
            object_id: ObjectId::NULL,
            character_kind: CharacterKind::default(),
            health_point: HealthPoint::default(),
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            velocity: [0.0, 0.0, 0.0],
            action_state: ActionState::default(),
            action_state_timer: ActionStateTimer::default(),
            movement_state: MovementState::default(),
            movement_state_timer: MovementStateTimer::default(),
            view_state: ViewState::default(),
            view_state_timer: ViewStateTimer::default(),
            view_rotation: LatLon::default(),
        }
    }
}

impl TryFromBigEndian for Player {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(Player)
        );

        // 오브젝트 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = ObjectId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let object_id = ObjectId::try_from_big_endian_bytes(data)?;

        // 캐릭터 종류를 가져옵니다.
        offset = offset + size;
        size = CharacterKind::byte_size();
        data = &bytes[offset..offset + size];
        let character_kind = CharacterKind::try_from_big_endian_bytes(data)?;

        // 체력을 가져옵니다.
        offset = offset + size;
        size = HealthPoint::byte_size();
        data = &bytes[offset..offset + size];
        let health_point = HealthPoint::try_from_big_endian_bytes(data)?;

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

        // 행동 상태를 가져옵니다.
        offset = offset + size;
        size = ActionState::byte_size();
        data = &bytes[offset..offset + size];
        let action_state = ActionState::try_from_big_endian_bytes(data)?;

        // 행동 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = ActionStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let action_state_timer = ActionStateTimer::from_big_endian_bytes(data);

        // 움직임 상태를 가져옵니다.
        offset = offset + size;
        size = MovementState::byte_size();
        data = &bytes[offset..offset + size];
        let movement_state = MovementState::try_from_big_endian_bytes(data)?;

        // 움직임 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = MovementStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let movement_state_timer = MovementStateTimer::from_big_endian_bytes(data);

        // 카메라 상태를 가져옵니다.
        offset = offset + size;
        size = ViewState::byte_size();
        data = &bytes[offset..offset + size];
        let view_state = ViewState::try_from_big_endian_bytes(data)?;

        // 카메라 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = ViewStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let view_state_timer = ViewStateTimer::from_big_endian_bytes(data);

        // 카메라 방향을 가져옵니다.
        offset = offset + size;
        size = LatLon::byte_size();
        data = &bytes[offset..offset + size];
        let view_rotation = LatLon::from_big_endian_bytes(data);

        Some(Self {
            object_id,
            character_kind,
            health_point,
            translation,
            rotation,
            velocity,
            action_state,
            action_state_timer,
            movement_state,
            movement_state_timer,
            view_state,
            view_state_timer,
            view_rotation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_player() {
        let origin = Player {
            object_id: ObjectId::new(3141592),
            character_kind: CharacterKind::MomoiOriginal,
            health_point: HealthPoint(2700.0),
            translation: [-1.0101, 2.3456, 1000.011],
            rotation: [0.1234, 1.99992, 0.08843, 1.0],
            velocity: [0.0, -0.1334, 0.5887],
            ..Default::default()
        };
        let bytes = origin.to_big_endian_bytes();
        let other = Player::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(Player::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
