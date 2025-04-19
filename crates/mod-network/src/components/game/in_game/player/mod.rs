//! 플레이어와 관련된 코드를 관리합니다.
//!

mod attack;
mod health;

use crate::components::{
    ActionState, ActionStateTimer, BigEndian, CharacterKind, LatLon, MovementState,
    MovementStateTimer, Team, TryFromBigEndian, UserAccount, ViewState, ViewStateTimer,
};

pub use self::{attack::*, health::*};

/// 게임 진행 단계일 때 플레이어 데이터를 저장합니다.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayPhasePlayer {
    /// 사용자 계정 데이터
    pub account: UserAccount,

    /// 플레이어 캐릭터 종류
    pub character_kind: CharacterKind,
    /// 플레이어 총알 정보
    pub remaining_bullet: RemainingBullet,
    /// 플레이어 최대 체력
    pub max_health_point: MaxHealthPoint,
    /// 플레이어 캐릭터 체력
    pub health_point: HealthPoint,
    /// 플레이어 캐릭터의 월드 공간 위치
    pub translation: [f32; 3],
    /// 플레이어 캐릭터가 바라보는 월드 공간 방향  
    /// ※ 캐릭터가 움직이는 방향과 다를 수 있습니다.
    pub rotation: [f32; 4],

    /// 여러 자료형의 데이터를 저장한 비트 필드입니다.  
    /// 아래 데이터가 포함됩니다.
    /// - Team (1bit): 플레이어가 속한 팀의 종류를 나타냄
    /// - ActionState (4bit): 플레이어 캐릭터의 행동 상태를 나타냄
    /// - MovementState (3bit): 플레이어 캐릭터의 움직임 상태를 나타냄
    /// - ViewState (2bit): 플레이어 캐릭터의 카메라 시야 상태를 나타냄
    ///
    pub bitfield: u16,
    /// 플레이어 캐릭터의 행동 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 플레이어 캐릭터의 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
    /// 플레이어 카메라 상태 타이머
    pub view_state_timer: ViewStateTimer,
    /// 플레이어 카메라가 캐릭터를 중심으로 바라보는 방향
    pub view_rotation: LatLon,
}

impl PlayPhasePlayer {
    /// 새로운 플레이어 데이터를 생성합니다.  
    /// `force`가 `true`인 경우 클라이언트는 데이터를 덮어씌웁니다.
    pub fn new(
        account: UserAccount,
        character_kind: CharacterKind,
        remaining_bullet: RemainingBullet,
        max_health_point: MaxHealthPoint,
        health_point: HealthPoint,
        translation: [f32; 3],
        rotation: [f32; 4],
        team: Team,
        action_state: ActionState,
        action_state_timer: ActionStateTimer,
        movement_state: MovementState,
        movement_state_timer: MovementStateTimer,
        view_state: ViewState,
        view_state_timer: ViewStateTimer,
        view_rotation: LatLon,
    ) -> Self {
        let team_field = (team as u16) << 9;
        let action_state_field = (action_state as u16) << 5;
        let movement_state_field = (movement_state as u16) << 2;
        let view_state_field = (view_state as u16) << 0;
        let bitfield = team_field | action_state_field | movement_state_field | view_state_field;

        Self {
            account,
            character_kind,
            remaining_bullet, 
            max_health_point,
            health_point,
            translation,
            rotation,
            bitfield,
            action_state_timer,
            movement_state_timer,
            view_state_timer,
            view_rotation,
        }
    }

    /// 플레이어 캐릭터 종류를 설정합니다.
    pub fn with_character(&mut self, character_kind: CharacterKind) -> &mut Self {
        self.character_kind = character_kind;
        self
    }

    /// 플레이어 캐릭터 체력을 설정합니다.
    pub fn with_health_point(&mut self, health_point: HealthPoint) -> &mut Self {
        self.health_point = health_point;
        self
    }

    /// 플레이어가 속한 팀을 설정합니다.
    pub fn with_team(&mut self, team: Team) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x1 << 9)) | (team as u16) << 9;
        self
    }

    /// 플레이어가 속한 팀을 가져옵니다.
    pub fn team(&self) -> Team {
        // Safe: 값이 범위를 벗어나지 않음
        unsafe {
            let val = ((self.bitfield >> 9) & 0x1) as u8;
            Team::new(val).unwrap_unchecked()
        }
    }

    /// 플레이어 캐릭터의 월드 공간 위치를 설정합니다.
    pub fn with_translation<T>(&mut self, translation: T) -> &mut Self
    where
        T: Into<[f32; 3]>,
    {
        self.translation = translation.into();
        self
    }

    /// 플레이어 캐릭터가 월드 공간에서 바라보는 방향을 설정합니다.  
    /// ※ 캐릭터가 움직이는 방향과 다를 수 있습니다.
    pub fn with_rotation<T>(&mut self, rotation: T) -> &mut Self
    where
        T: Into<[f32; 4]>,
    {
        self.rotation = rotation.into();
        self
    }

    /// 플레이어 캐릭터 행동 상태를 설정합니다.
    pub fn with_action_state(&mut self, action_state: ActionState) -> &mut Self {
        self.bitfield = (self.bitfield & !(0xF << 5)) | (action_state as u16) << 5;
        self
    }

    /// 플레이어 캐릭터 행동 상태를 가져옵니다.
    pub fn action_state(&self) -> ActionState {
        let val = ((self.bitfield >> 5) & 0xF) as u8;
        ActionState::new(val).unwrap_or_default()
    }

    /// 플레이어 캐릭터 행동 상태 타이머를 설정합니다.
    pub fn with_action_state_timer(&mut self, action_state_timer: ActionStateTimer) -> &mut Self {
        self.action_state_timer = action_state_timer;
        self
    }

    /// 플레이어 캐릭터 움직임 상태를 설정합니다.
    pub fn with_movement_state(&mut self, movement_state: MovementState) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x7 << 2)) | (movement_state as u16) << 2;
        self
    }

    /// 플레이어 캐릭터 움직임 상태를 가져옵니다.
    pub fn movement_state(&self) -> MovementState {
        let val = ((self.bitfield >> 2) & 0x7) as u8;
        MovementState::new(val).unwrap_or_default()
    }

    /// 플레이어 캐릭터 움직임 상태 타이머를 설정합니다.
    pub fn with_movement_state_timer(
        &mut self,
        movement_state_timer: MovementStateTimer,
    ) -> &mut Self {
        self.movement_state_timer = movement_state_timer;
        self
    }

    /// 플레이어 카메라 상태를 설정합니다.
    pub fn with_view_state(&mut self, view_state: ViewState) -> &mut Self {
        self.bitfield = (self.bitfield & !(0x3 << 0)) | (view_state as u16) << 0;
        self
    }

    /// 플레이어 카메라 상태를 가져옵니다.
    pub fn view_state(&self) -> ViewState {
        // Safe: 값이 범위를 벗어나지 않음
        unsafe {
            let val = ((self.bitfield >> 0) & 0x3) as u8;
            ViewState::new(val).unwrap_unchecked()
        }
    }

    /// 플레이어 캐릭터 카메라 상태 타이머를 설정합니다.
    pub fn with_view_state_timer(&mut self, view_state_timer: ViewStateTimer) -> &mut Self {
        self.view_state_timer = view_state_timer;
        self
    }

    /// 플레이어 캐릭터 카메라 회전 각도를 설정합니다.
    pub fn with_view_rotation(&mut self, view_rotation: LatLon) -> &mut Self {
        self.view_rotation = view_rotation;
        self
    }
}

impl BigEndian for PlayPhasePlayer {
    fn byte_size() -> usize {
        UserAccount::byte_size()
            + CharacterKind::byte_size()
            + RemainingBullet::byte_size()
            + MaxHealthPoint::byte_size()
            + HealthPoint::byte_size()
            + <[f32; 3]>::byte_size()
            + <[f32; 4]>::byte_size()
            + u16::byte_size()
            + ActionStateTimer::byte_size()
            + MovementStateTimer::byte_size()
            + ViewStateTimer::byte_size()
            + LatLon::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let mut bytes = Vec::with_capacity(Self::byte_size());
        bytes.extend_from_slice(&self.account.to_big_endian_bytes());
        bytes.extend_from_slice(&self.character_kind.to_big_endian_bytes());
        bytes.extend_from_slice(&self.remaining_bullet.to_big_endian_bytes());
        bytes.extend_from_slice(&self.max_health_point.to_big_endian_bytes());
        bytes.extend_from_slice(&self.health_point.to_big_endian_bytes());
        bytes.extend_from_slice(&self.translation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.rotation.to_big_endian_bytes());
        bytes.extend_from_slice(&self.bitfield.to_big_endian_bytes());
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
                stringify!(PlayPhasePlayer)
            );
        }

        bytes
    }
}

impl Default for PlayPhasePlayer {
    fn default() -> Self {
        Self {
            account: UserAccount::default(),
            character_kind: CharacterKind::default(),
            remaining_bullet: RemainingBullet::default(),
            max_health_point: MaxHealthPoint::default(),
            health_point: HealthPoint::default(),
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            bitfield: 0x0000,
            action_state_timer: ActionStateTimer::default(),
            movement_state_timer: MovementStateTimer::default(),
            view_state_timer: ViewStateTimer::default(),
            view_rotation: LatLon::default(),
        }
    }
}

impl TryFromBigEndian for PlayPhasePlayer {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        // 바이트 배열의 크기가 다른지 확인한다.
        assert_eq!(
            bytes.len(),
            Self::byte_size(),
            "the size of the byte array and the size of the `{}` are different!",
            stringify!(PlayPhasePlayer)
        );

        // 사용자 계정 데이터를 가져옵니다.
        let mut offset = 0;
        let mut size = UserAccount::byte_size();
        let mut data = &bytes[offset..offset + size];
        let account = UserAccount::from_big_endian_bytes(data);

        // 플레이어 캐릭터 종류를 가져옵니다.
        offset = offset + size;
        size = CharacterKind::byte_size();
        data = &bytes[offset..offset + size];
        let character_kind = CharacterKind::try_from_big_endian_bytes(data)?;

        // 남은 총알 개수 데이터를 가져옵니다.
        offset = offset + size;
        size = RemainingBullet::byte_size();
        data = &bytes[offset..offset + size];
        let remaining_bullet = RemainingBullet::from_big_endian_bytes(data);

        // 플레이어 캐릭터 최대 체력을 가져옵니다.
        offset = offset + size;
        size = HealthPoint::byte_size();
        data = &bytes[offset..offset + size];
        let max_health_point = MaxHealthPoint::try_from_big_endian_bytes(data)?;

        // 플레이어 캐릭터 체력을 가져옵니다.
        offset = offset + size;
        size = HealthPoint::byte_size();
        data = &bytes[offset..offset + size];
        let health_point = HealthPoint::from_big_endian_bytes(data);

        // 플레이어 캐릭터 월드 공간 위치 데이터를 가져옵니다.
        offset = offset + size;
        size = <[f32; 3]>::byte_size();
        data = &bytes[offset..offset + size];
        let translation = <[f32; 3]>::from_big_endian_bytes(data);

        // 플레이어 캐릭터가 바라보는 월드 공간 방향을 가져옵니다.
        offset = offset + size;
        size = <[f32; 4]>::byte_size();
        data = &bytes[offset..offset + size];
        let rotation = <[f32; 4]>::from_big_endian_bytes(data);

        // 비트 필드를 가져옵니다.
        offset = offset + size;
        size = u16::byte_size();
        data = &bytes[offset..offset + size];
        let bitfield = u16::from_big_endian_bytes(data);

        // 플레이어 캐릭터 행동 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = ActionStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let action_state_timer = ActionStateTimer::from_big_endian_bytes(data);

        // 플레이어 캐릭터 움직임 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = MovementStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let movement_state_timer = MovementStateTimer::from_big_endian_bytes(data);

        // 플레이어 카메라 상태 타이머를 가져옵니다.
        offset = offset + size;
        size = ViewStateTimer::byte_size();
        data = &bytes[offset..offset + size];
        let view_state_timer = ViewStateTimer::from_big_endian_bytes(data);

        // 플레이어 카메라가 캐릭터를 중심으로 바라보는 방향을 가져옵니다.
        offset = offset + size;
        size = LatLon::byte_size();
        data = &bytes[offset..offset + size];
        let view_rotation = LatLon::from_big_endian_bytes(data);

        Some(Self {
            account,
            character_kind,
            remaining_bullet, 
            max_health_point,
            health_point,
            translation,
            rotation,
            bitfield,
            action_state_timer,
            movement_state_timer,
            view_state_timer,
            view_rotation,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use crate::components::{UserId, UserName};

    use super::*;

    #[test]
    fn test_play_phase_player() {
        let id = UserId::new(1314311);
        let name = UserName::from_str("Aris");
        let account = UserAccount::new(id, name);
        let origin = PlayPhasePlayer::new(
            account,
            CharacterKind::MomoiOriginal,
            RemainingBullet::new(10, 7),
            MaxHealthPoint::new(NonZeroU16::new(1234).unwrap()),
            HealthPoint(1324),
            [12.0, 34.123, 1.23423],
            [1.243214, 0.51251512, 0.1324131, 0.34151512],
            Team::Red,
            ActionState::Aiming,
            ActionStateTimer(1.2432),
            MovementState::InPlaceLanding,
            MovementStateTimer(1.2451351),
            ViewState::ZoomIn,
            ViewStateTimer(1.15312),
            LatLon {
                lat: 1.234151,
                lon: 2.1341515,
            },
        );
        let bytes = origin.to_big_endian_bytes();
        let other = PlayPhasePlayer::from_big_endian_bytes(&bytes);

        // 원본과 일치하는지 확인
        assert_eq!(origin, other);
    }
}
