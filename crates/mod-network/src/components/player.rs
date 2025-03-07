use std::{fmt, mem};

use super::{
    ActionStateTimer, BigEndian, CharacterKind, CompressedState, HealthPoint, LatLon,
    MovementStateTimer, TryFromBigEndian, UserId, ViewStateTimer,
};

/// 사용자 닉네임 크기입니다.
const MAX_NAME_BUF_SIZE: usize = 16;
/// 최대 사용자 닉네임 길이입니다.
pub const MAX_NAME_LEN: usize = MAX_NAME_BUF_SIZE - 1;

/// 서버에서 클라이언트로 플레이어 캐릭터 데이터를 보내는데 사용되는 구조체
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Player {
    /// 사용자 식별자
    pub user_id: UserId,
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
    /// 플레이어의 압축된 상태 데이터 입니다.
    pub compressed_state: CompressedState,
    /// 플레이어 캐릭터의 행동 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 플레이어 캐릭터의 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
    /// 플레이어 카메라 상태 타이머
    pub view_state_timer: ViewStateTimer,
    /// 플레이어 카메라가 캐릭터를 중심으로 바라보는 방향
    pub view_rotation: LatLon,
}

impl BigEndian for Player {
    fn byte_size() -> usize {
        UserId::byte_size()
            + CharacterKind::byte_size()
            + HealthPoint::byte_size()
            + <[f32; 3]>::byte_size()
            + <[f32; 4]>::byte_size()
            + <[f32; 3]>::byte_size()
            + CompressedState::byte_size()
            + ActionStateTimer::byte_size()
            + MovementStateTimer::byte_size()
            + ViewStateTimer::byte_size()
            + LatLon::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.user_id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.health_point.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.velocity.to_big_endian_bytes());
        bytes.extend_from_slice(&self.compressed_state.to_big_endian_bytes());
        bytes.extend_from_slice(&self.action_state_timer.to_big_endian_bytes());
        bytes.extend_from_slice(&self.movement_state_timer.to_big_endian_bytes());
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
            user_id: UserId::default(),
            character_kind: CharacterKind::default(),
            health_point: HealthPoint::default(),
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            velocity: [0.0, 0.0, 0.0],
            compressed_state: CompressedState::default(),
            action_state_timer: ActionStateTimer::default(),
            movement_state_timer: MovementStateTimer::default(),
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

        // 사용자 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

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

        // 압축된 상태를 가져옵니다.
        offset = offset + size;
        size = CompressedState::byte_size();
        data = &bytes[offset..offset + size];
        let compressed_state = CompressedState::from_big_endian_bytes(data);

        // 행동 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = ActionStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let action_state_timer = ActionStateTimer::from_big_endian_bytes(data);

        // 움직임 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = MovementStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let movement_state_timer = MovementStateTimer::from_big_endian_bytes(data);

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
            user_id,
            character_kind,
            health_point,
            translation,
            rotation,
            velocity,
            compressed_state,
            action_state_timer,
            movement_state_timer,
            view_state_timer,
            view_rotation,
        })
    }
}

/// UTF-16 형식의 사용자 닉네임 데이터입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserName([u16; MAX_NAME_BUF_SIZE]);

impl UserName {
    /// 비어있는 사용자 닉네임입니다.
    pub const EMPTY: Self = Self([0; MAX_NAME_BUF_SIZE]);

    /// 문자열로부터 사용자 닉네임을 생성합니다.
    ///
    /// 이 함수는 자동으로 문자열 사이의 공백을 제거하고, 닉네임 길이만큼 문자열을 자릅니다.
    ///
    pub fn new<T: AsRef<str>>(s: T) -> Self {
        let mut iter = s.as_ref().trim().encode_utf16();
        let mut name = [0; MAX_NAME_BUF_SIZE];
        for i in 0..MAX_NAME_LEN {
            match iter.next() {
                Some(ch) => name[i] = ch,
                None => break,
            };
        }
        Self(name)
    }
}

impl BigEndian for UserName {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        const SIZE: usize = mem::size_of::<u16>();
        let mut name = [0; MAX_NAME_BUF_SIZE];
        let mut offset = 0;
        for i in 0..MAX_NAME_LEN {
            let data = &bytes[offset..offset + SIZE];
            name[i] = u16::from_big_endian_bytes(data);
            offset += SIZE;
        }

        Self(name)
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        let name = &self.0;
        for i in 0..MAX_NAME_BUF_SIZE {
            bytes.extend_from_slice(&name[i].to_big_endian_bytes());
        }
        bytes
    }
}

impl Default for UserName {
    fn default() -> Self {
        Self([0; MAX_NAME_BUF_SIZE])
    }
}

impl fmt::Display for UserName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &String::from_utf16_lossy(&self.0))
    }
}

/// 사용자 정보입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct User {
    /// 사용자 식별자
    id: UserId,
    /// 사용자 닉네임 (UTF-16)
    name: UserName,
}

impl User {
    /// 비어있는 사용자 데이터입니다.
    pub const EMPTY: Self = Self { id: UserId::NULL, name: UserName::EMPTY };

    pub fn new(id: UserId, name: UserName) -> Self {
        Self { id, name }
    }

    /// 사용자 식별자를 반환합니다.
    pub fn id(&self) -> UserId {
        self.id
    }

    /// 사용자 이름을 반환합니다.
    pub fn name(&self) -> &UserName {
        &self.name
    }
}

impl BigEndian for User {
    fn byte_size() -> usize {
        UserId::byte_size() + UserName::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(User)
        );

        // 사용자 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserId::byte_size();
        let mut data = &bytes[offset..offset + size];
        let user_id = UserId::from_big_endian_bytes(data);

        // 사용자 닉네임을 가져옵니다.
        offset = offset + size;
        size = UserName::byte_size();
        data = &bytes[offset..offset + size];
        let name = UserName::from_big_endian_bytes(data);

        Self { id: user_id, name }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.id.to_big_endian_bytes());
        bytes.extend_from_slice(&self.name.to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(User)
            );
        }

        bytes
    }
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: UserId::default(),
            name: UserName::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_test_player() {
        let origin = Player {
            user_id: UserId::new(3141592),
            character_kind: CharacterKind::MomoiOriginal,
            health_point: HealthPoint(2700),
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

    #[test]
    fn validation_test_user() {
        let origin = User {
            id: UserId::new(3141592),
            name: UserName::new("Hello안녕!"),
        };
        let bytes = origin.to_big_endian_bytes();
        let other = User::from_big_endian_bytes(&bytes);

        // 바이트 배열 크기가 같은지 확인
        assert_eq!(User::byte_size(), bytes.len());
        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
