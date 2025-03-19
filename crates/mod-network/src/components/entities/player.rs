use std::{cmp, fmt, hash};

use crate::components::{
    ActionState, ActionStateTimer, BigEndian, CharacterKind, HealthPoint, LatLon, MovementState,
    MovementStateTimer, TryFromBigEndian, UserId, ViewState, ViewStateTimer,
};

/// 사용자 닉네임 문자열 버퍼의 크기입니다.
const MAX_NAME_BUF_SIZE: usize = 16;
/// 사용자 닉네임의 최대 길이입니다.
pub const MAX_NAME_LEN: usize = MAX_NAME_BUF_SIZE - 1;

/// UTF-16 형식의 사용자 닉네임 문자열
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserName {
    len: usize,
    buffer: [u16; MAX_NAME_BUF_SIZE]
}

impl UserName {
    /// 문자열 요소의 크기입니다.
    pub const ELEMENT_SIZE: usize = core::mem::size_of::<u16>();

    /// 문자열로부터 사용자 닉네임을 생성합니다.
    ///
    /// 이 함수는 자동으로 문자열 사이의 공백을 제거하고, 닉네임 길이만큼 문자열을 자릅니다.
    ///
    pub fn new<T: AsRef<str>>(s: T) -> Self {
        let mut iter = s.as_ref().trim().encode_utf16();
        let mut buffer = [0; MAX_NAME_BUF_SIZE];
        let mut len = 0;
        for i in 0..MAX_NAME_LEN {
            match iter.next() {
                Some(ch) => {
                    buffer[i] = ch;
                    len += 1;
                },
                None => break,
            };
        }

        Self { len, buffer }
    }
}

impl BigEndian for UserName {
    fn byte_size() -> usize {
        u8::byte_size() + Self::ELEMENT_SIZE * MAX_NAME_BUF_SIZE
    }
    
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        // 바이트 배열 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(UserName)
        );

        // 문자열 길이를 가져옵니다.
        let mut offset = 0;
        let mut size = u8::byte_size();
        let mut data = &bytes[offset..offset + size];
        let len = u8::from_big_endian_bytes(data) as usize;

        // 문자열 버퍼를 가져옵니다.
        let mut buffer = [0; MAX_NAME_BUF_SIZE];
        for i in 0..MAX_NAME_LEN.min(len) {
            offset = offset + size;
            size = Self::ELEMENT_SIZE;
            data = &bytes[offset..offset + size];
            buffer[i] = u16::from_big_endian_bytes(data);
        }

        Self { len, buffer }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&(self.len as u8).to_big_endian_bytes());
        for i in 0..MAX_NAME_BUF_SIZE {
            bytes.extend_from_slice(&self.buffer[i].to_big_endian_bytes());
        }
        bytes
    }
}

impl Default for UserName {
    fn default() -> Self {
        Self {
            len: 0,
            buffer: [0; MAX_NAME_BUF_SIZE],
        }
    }
}

impl fmt::Display for UserName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &String::from_utf16_lossy(&self.buffer[..self.len]))
    }
}

/// 사용자 정보
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserInfo {
    /// 사용자 식별자
    pub uid: UserId,
    /// 사용자 닉네임
    pub name: UserName,
}

impl UserInfo {
    /// 새로운 사용자 정보를 생성합니다.
    pub fn new(id: UserId, name: UserName) -> Self {
        Self { uid: id, name }
    }
}

impl BigEndian for UserInfo {
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

        Self { uid: user_id, name }
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.uid.to_big_endian_bytes());
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

impl Default for UserInfo {
    fn default() -> Self {
        Self {
            uid: UserId::default(),
            name: UserName::default(),
        }
    }
}

/// 플레이어가 속한 팀의 종류
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Team {
    Blue = 0,
    Red = 1,
}

impl Team {
    /// 주어진 정수로 부터 `Team`을 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Team::Blue),
            1 => Some(Team::Red),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(Team),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for Team {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for Team {
    fn default() -> Self {
        Self::Blue
    }
}

impl TryFromBigEndian for Team {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 커스텀 게임 대기실에서 플레이어의 권한
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
    User = 0,
    Admin = 1,
}

impl Permission {
    /// 주어진 정수로 부터 `Permission`을 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Permission::User),
            1 => Some(Permission::Admin),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(Permission),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for Permission {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for Permission {
    fn default() -> Self {
        Self::User
    }
}

impl TryFromBigEndian for Permission {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 커스텀 게임 대기실에서 플레이어의 상태  
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CustomGameStatus {
    Wait = 0,
    Ready = 1,
}

impl CustomGameStatus {
    /// 주어진 정수로 부터 `CustomGameStatus`를 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(CustomGameStatus::Wait),
            1 => Some(CustomGameStatus::Ready),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(CustomGamePlayerStatus),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for CustomGameStatus {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for CustomGameStatus {
    fn default() -> Self {
        Self::Wait
    }
}

impl TryFromBigEndian for CustomGameStatus {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 서버에서 클라이언트로 보내는 커스텀 게임에 참여한 플레이어 정보
///
/// # Note
/// 아래 데이터는 1byte로 압축되어 보내집니다.
/// - team (8bit -> 1bit)
/// - status (8bit -> 2bit)
/// - permission (8bit -> 1bit)
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomGamePlayer {
    /// 플레이어의 사용자 정보
    pub info: UserInfo,
    /// 플레이어가 속한 팀의 종류
    pub team: Team,
    /// 플레이어 상태
    pub status: CustomGameStatus,
    /// 플레이어의 권한
    pub permission: Permission,
}

impl CustomGamePlayer {
    /// 일부 맴버 변수의 데이터를 압축합니다.
    fn compress(&self) -> u8 {
        // +------+-------------------+---------------+-------------+
        // | 3bit | permission (1bit) | status (2bit) | team (1bit) |
        // +------+-------------------+---------------+-------------+
        //
        let permission_bit = (self.permission as u8) << 4;
        let status_bit = (self.status as u8) << 1;
        let team_bit = (self.team as u8) << 0;

        permission_bit | status_bit | team_bit
    }

    /// 압축된 데이터를 원래 데이터로 복원합니다.  
    /// 원래 데이터로 복원에 실패할 경우 `None`을 반환합니다.
    fn try_decompress(bit: u8) -> Option<(Permission, CustomGameStatus, Team)> {
        let val = (bit >> 4) & 0x1;
        let permission = Permission::new(val)?;

        let val = (bit >> 1) & 0x3;
        let status = CustomGameStatus::new(val)?;

        let val = (bit >> 0) & 0x1;
        let team = Team::new(val)?;

        Some((permission, status, team))
    }
}

impl BigEndian for CustomGamePlayer {
    fn byte_size() -> usize {
        UserInfo::byte_size() + u8::byte_size() // 압축된 데이터 크기
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.info.to_big_endian_bytes());
        bytes.extend_from_slice(&self.compress().to_big_endian_bytes());

        // 바이트 배열 유효성 검증
        if cfg!(feature = "check-validation") {
            assert_eq!(
                bytes.len(),
                Self::byte_size(),
                "the size of the byte array and the size of the `{}` are different!",
                stringify!(CustomGamePlayer)
            );
        }

        bytes
    }
}

impl Default for CustomGamePlayer {
    fn default() -> Self {
        Self {
            info: UserInfo::default(),
            team: Team::default(),
            status: CustomGameStatus::default(),
            permission: Permission::default(),
        }
    }
}

impl TryFromBigEndian for CustomGamePlayer {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(CustomGamePlayer)
        );

        // 사용자 정보를 가져옵니다.
        let mut offset = 0;
        let mut size = UserInfo::byte_size();
        let mut data = &bytes[offset..offset + size];
        let info = UserInfo::from_big_endian_bytes(data);

        // 압축된 데이터를 가져옵니다.
        offset = offset + size;
        size = u8::byte_size();
        data = &bytes[offset..offset + size];
        let (permission, status, team) = Self::try_decompress(u8::from_big_endian_bytes(data))?;

        Some(Self {
            info,
            team,
            status,
            permission,
        })
    }
}

impl cmp::Ord for CustomGamePlayer {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.info.uid.cmp(&other.info.uid)
    }
}

impl cmp::PartialOrd for CustomGamePlayer {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.info.uid.partial_cmp(&other.info.uid)
    }
}

impl hash::Hash for CustomGamePlayer {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.info.uid.hash(state);
    }
}

/// 인게임에서 플레이어의 상태
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InGameStatus {
    Connect = 0,
    Disconnect = 1,
    Live = 2,
    Die = 3,
}

impl InGameStatus {
    /// 주어진 정수로 부터 `InGameStatus`를 생성합니다.  
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(InGameStatus::Connect),
            1 => Some(InGameStatus::Disconnect),
            2 => Some(InGameStatus::Live),
            3 => Some(InGameStatus::Die),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(InGameStatus),
                    val
                );
                None
            }
        }
    }
}

impl BigEndian for InGameStatus {
    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("out of bounds")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        let index = *self as u8;
        index.to_big_endian_bytes()
    }
}

impl Default for InGameStatus {
    fn default() -> Self {
        Self::Connect
    }
}

impl TryFromBigEndian for InGameStatus {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        Self::new(u8::from_big_endian_bytes(bytes))
    }
}

/// 서버에서 클라이언트로 보내는 인게임 플레이어 정보
///
/// # Note
/// 아래 데이터는 16bit로 압축되어 보내집니다.
/// - team (8bit -> 1bit),
/// - status (8bit -> 3bit),
/// - action_state (8bit -> 4bit),
/// - movement_state (8bit -> 3bit),
/// - view_state (8bit -> 2bit)
///
#[derive(Debug, Clone, PartialEq)]
pub struct InGamePlayer {
    /// 플레이어의 사용자 정보
    pub info: UserInfo,
    /// 플레이어가 속한 팀
    pub team: Team,
    /// 플레이어의 상태
    pub status: InGameStatus,

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
    /// 플레이어 캐릭터의 움직임 상태
    pub movement_state: MovementState,
    /// 플레이어 카메라 상태
    pub view_state: ViewState,
    /// 플레이어 캐릭터의 행동 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 플레이어 캐릭터의 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
    /// 플레이어 카메라 상태 타이머
    pub view_state_timer: ViewStateTimer,
    /// 플레이어 카메라가 캐릭터를 중심으로 바라보는 방향
    pub view_rotation: LatLon,
}

impl InGamePlayer {
    /// 일부 맴버 변수의 데이터를 압축합니다.
    fn compress(&self) -> u16 {
        // +------+-------------+---------------+---------------------+-----------------------+-------------------+
        // | 3bit | team (1bit) | status (3bit) | action_state (4bit) | movement_state (3bit) | view_state (2bit) |
        // +------+-------------+---------------+---------------------+-----------------------+-------------------+
        //
        let team_bit = (self.team as u16) << 12;
        let status_bit = (self.status as u16) << 9;
        let action_bit = (self.action_state as u16) << 5;
        let movement_bit = (self.movement_state as u16) << 2;
        let view_bit = (self.view_state as u16) << 0;

        team_bit | status_bit | action_bit | movement_bit | view_bit
    }

    /// 압축된 데이터를 원래 데이터로 복원합니다.  
    /// 원래 데이터로 복원에 실패할 경우 `None`을 반환합니다.
    fn try_decompress(
        bit: u16,
    ) -> Option<(Team, InGameStatus, ActionState, MovementState, ViewState)> {
        let val = (bit >> 12) & 0x1;
        let team = Team::new(val as u8)?;

        let val = (bit >> 9) & 0x7;
        let status = InGameStatus::new(val as u8)?;

        let val = (bit >> 5) & 0xF;
        let action_state = ActionState::new(val as u8)?;

        let val = (bit >> 2) & 0x7;
        let movement_state = MovementState::new(val as u8)?;

        let val = (bit >> 0) & 0x3;
        let view_state = ViewState::new(val as u8)?;

        Some((team, status, action_state, movement_state, view_state))
    }
}

impl BigEndian for InGamePlayer {
    fn byte_size() -> usize {
        UserInfo::byte_size()
            + CharacterKind::byte_size()
            + HealthPoint::byte_size()
            + <[f32; 3]>::byte_size()
            + <[f32; 4]>::byte_size()
            + <[f32; 3]>::byte_size()
            + u16::byte_size() // 압축된 데이터 크기
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
        bytes.extend_from_slice(&self.info.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.health_point.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.velocity.to_big_endian_bytes());
        bytes.extend_from_slice(&self.compress().to_big_endian_bytes());
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
                stringify!(InGamePlayer)
            );
        }

        bytes
    }
}

impl Default for InGamePlayer {
    fn default() -> Self {
        // object_id의 기본 값은 NULL이어야 합니다.
        Self {
            info: UserInfo::default(),
            team: Team::default(),
            status: InGameStatus::default(),
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

impl TryFromBigEndian for InGamePlayer {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(InGamePlayer)
        );

        // 사용자 식별자를 가져옵니다.
        let mut offset = 0;
        let mut size = UserInfo::byte_size();
        let mut data = &bytes[offset..offset + size];
        let info = UserInfo::from_big_endian_bytes(data);

        // 사용자 정보를 가져옵니다.
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

        // 압축된 데이터를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let (team, status, action_state, movement_state, view_state) =
            Self::try_decompress(u16::from_big_endian_bytes(data))?;

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
            info,
            team,
            status,
            character_kind,
            health_point,
            translation,
            rotation,
            velocity,
            action_state,
            movement_state,
            view_state,
            action_state_timer,
            movement_state_timer,
            view_state_timer,
            view_rotation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_info() {
        let origin = UserInfo {
            uid: UserId::new(3141592),
            name: UserName::new("Hello안녕!"),
        };
        let bytes = origin.to_big_endian_bytes();
        let other = UserInfo::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_custom_game_player() {
        let id = UserId::new(123576);
        let name = UserName::new("Hello,안녕!");
        let info = UserInfo::new(id, name);
        let team = Team::Red;
        let status = CustomGameStatus::Ready;
        let permission = Permission::Admin;

        let origin = CustomGamePlayer {
            info,
            team,
            status,
            permission,
        };
        let bytes = origin.to_big_endian_bytes();
        let other = CustomGamePlayer::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }

    #[test]
    fn test_in_game_player() {
        let info = UserInfo::new(UserId::new(3141592), UserName::new("Hello,안녕!"));

        let origin = InGamePlayer {
            info,
            team: Team::Blue,
            status: InGameStatus::Live,
            character_kind: CharacterKind::MomoiOriginal,
            health_point: HealthPoint(2700),
            translation: [-1.0101, 2.3456, 1000.011],
            rotation: [0.1234, 1.99992, 0.08843, 1.0],
            velocity: [0.0, -0.1334, 0.5887],
            action_state: ActionState::AimOff,
            movement_state: MovementState::Idle,
            view_state: ViewState::ZoomOut,
            ..Default::default()
        };
        let bytes = origin.to_big_endian_bytes();
        let other = InGamePlayer::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
