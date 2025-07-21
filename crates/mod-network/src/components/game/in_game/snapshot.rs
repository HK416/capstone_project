//! 스냅샷 데이터와 관련된 코드를 관리합니다.
//!

use crate::components::{
    ActionState, ActionStateTimer, BigEndian, BulletData, HeldInput, InputKind, InputStateTimer,
    LatLon, MovementState, MovementStateTimer, MovingDirection, SkillCostData, TryFromBigEndian,
    Velocity,
};

/// Epsilon
pub const LAT_LON_EPSILON: f32 = 3f32.to_radians();
/// 강제 스냅샷 저장 주기
pub const MAX_SNAPSHOT_INTERVAL_MS: u32 = 33;

/// 키 입력 이벤트 데이터입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputEvent {
    KeyPress(InputKind),
    KeyRelease(InputKind),
}

impl InputEvent {
    const INPUT_BIT_MASK: u8 = 0x7F;
    const INPUT_SHIFT: usize = 0;
    const PRESS_BIT_MASK: u8 = 0x01;
    const PRESS_SHIFT: usize = 7;

    /// 키 눌림 여부를 반환합니다.
    pub const fn is_pressed(self) -> bool {
        match self {
            InputEvent::KeyPress(_) => true,
            InputEvent::KeyRelease(_) => false,
        }
    }

    /// 입력된 키의 종류를 반환합니다.
    pub const fn input_kind(self) -> InputKind {
        match self {
            InputEvent::KeyPress(input_kind) => input_kind,
            InputEvent::KeyRelease(input_kind) => input_kind,
        }
    }
}

impl BigEndian for InputEvent {
    fn byte_size() -> usize {
        u8::byte_size()
    }

    fn from_big_endian_bytes(bytes: &[u8]) -> Self {
        Self::try_from_big_endian_bytes(bytes).expect("invalid data!")
    }

    fn to_big_endian_bytes(&self) -> Vec<u8> {
        // 바이트 스트림을 생성합니다.
        let pressed = ((self.is_pressed() as u8) & Self::PRESS_BIT_MASK) << Self::PRESS_SHIFT;
        let input = ((self.input_kind() as u8) & Self::INPUT_BIT_MASK) << Self::INPUT_SHIFT;
        let bits = pressed | input;
        bits.to_big_endian_bytes()
    }
}

impl TryFromBigEndian for InputEvent {
    fn try_from_big_endian_bytes(bytes: &[u8]) -> Option<Self> {
        let bits = u8::from_big_endian_bytes(bytes);
        let pressed = (bits >> Self::PRESS_SHIFT) & Self::PRESS_BIT_MASK == Self::PRESS_BIT_MASK;
        let input = InputKind::new((bits >> Self::INPUT_SHIFT) & Self::INPUT_BIT_MASK)?;
        Some(match pressed {
            true => Self::KeyPress(input),
            false => Self::KeyRelease(input),
        })
    }
}

/// 최대 입력키 이벤트의 개수입니다.
pub const MAX_INPUT_EVENTS: usize = 127 as usize;

/// 클라이언트 입력에 대한 스냅샷 데이터입니다.
#[derive(Debug, Clone, PartialEq)]
pub enum InputSnapshot {
    CameraOrientation {
        /// 게임 플레이 경과 시간
        play_elapsed_time_ms: u32,
        /// 위도 (latitude, pitch) - 상하 회전, 라디안 단위
        delta_lat: f32,
        /// 경도 (longitude, yaw) - 좌위 회전, 라디안 단위
        delta_lon: f32,
    },
    KeyEvent {
        /// 게임 플레이 경과 시간
        play_elapsed_time_ms: u32,
        /// 입력 키 이벤트 - 요소의 개수가 비어있거나, [`MAX_INPUT_EVENTS`]를 넘기면 안됩니다!
        events: Vec<InputEvent>,
    },
}

impl InputSnapshot {
    /// 게임 플레이 경과 시간을 설정합니다.
    pub fn set_play_elapsed_time_ms(&mut self, new_play_elapsed_time_ms: u32) {
        let play_elapsed_time_ms = match self {
            InputSnapshot::CameraOrientation {
                play_elapsed_time_ms,
                ..
            } => play_elapsed_time_ms,
            InputSnapshot::KeyEvent {
                play_elapsed_time_ms,
                ..
            } => play_elapsed_time_ms,
        };
        *play_elapsed_time_ms = new_play_elapsed_time_ms;
    }

    /// 게임 플레이 경과 시간을 반환합니다.
    pub const fn play_elapsed_time_ms(&self) -> u32 {
        match self {
            InputSnapshot::CameraOrientation {
                play_elapsed_time_ms,
                ..
            } => *play_elapsed_time_ms,
            InputSnapshot::KeyEvent {
                play_elapsed_time_ms,
                ..
            } => *play_elapsed_time_ms,
        }
    }
}

/// 최대 플레이어 스냅샷 데이터의 개수입니다.
pub const MAX_PLAYER_SNAPSHOTS: usize = 127;

/// 플레이어 스냅샷 데이터입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    /// 플레이 경과 시간
    pub play_elapsed_time_ms: u32,
    /// 행동 상태
    pub action_state: ActionState,
    /// 움직임 상태
    pub movement_state: MovementState,
    /// 행동 상태 타이머
    pub action_state_timer: ActionStateTimer,
    /// 움직임 상태 타이머
    pub movement_state_timer: MovementStateTimer,
    /// 총알 데이터
    pub bullet_data: BulletData,
    /// 스킬 코스트 데이터
    pub skill_cost_data: SkillCostData,
    /// 플레이어 카메라 각도
    pub latlon: LatLon,
    /// 플레이어 월드 공간 위치
    pub translation: glam::Vec3A,
    /// 플레이어 월드 공간 방향
    pub rotation: glam::Quat,
    /// 플레이어 월드 공간 이동 속도
    pub velocity: Velocity,
    /// 플레이어 월드 공간 이동 방향
    pub direction: MovingDirection,
    /// 입력 상태 타이머
    pub input_state_timer: InputStateTimer,
    /// 게임 입력 비트 플래그
    pub held_input: HeldInput,
    /// 무적 여부
    pub is_invincible: bool,
    /// 지면을 밟고 있는 여부
    pub is_grounded: bool,
}
