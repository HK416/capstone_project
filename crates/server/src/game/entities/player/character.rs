use mod_network::components::{
    ActionState, ActionStateTimer, CharacterAttributes, CharacterKind, MovementState,
    MovementStateTimer,
};

use crate::data::get_character_attributes;

/// 캐릭터 속성을 저장합니다.
#[derive(Debug, Clone)]
pub struct CharacterComponent {
    /// 플레이어 선택 캐릭터의 속성 데이터입니다.
    pub attributes: &'static CharacterAttributes,
    /// 플레이어 선택 캐릭터 종류입니다.
    pub character_kind: CharacterKind,

    /// 플레이어 캐릭터의 행동 상태입니다.
    pub action_state: ActionState,
    /// 플레이어 캐릭터의 이전 행동 상태입니다.
    pub prev_action_state: ActionState,
    /// 플레이어 캐릭터 행동 상태 타이머입니다.
    pub action_state_timer: ActionStateTimer,

    /// 플레이어 캐릭터의 움직임 상태입니다.
    pub movement_state: MovementState,
    /// 플레이어 캐릭터의 이전 움직임 상태입니다.
    pub prev_movement_state: MovementState,
    /// 플레이어 캐릭터 움직임 상태 타이머입니다.
    pub movement_state_timer: MovementStateTimer,
}

impl Default for CharacterComponent {
    fn default() -> Self {
        Self {
            attributes: get_character_attributes(CharacterKind::default()),
            character_kind: CharacterKind::default(),
            action_state: ActionState::default(),
            prev_action_state: ActionState::default(),
            action_state_timer: ActionStateTimer::default(),
            movement_state: MovementState::default(),
            prev_movement_state: MovementState::default(),
            movement_state_timer: MovementStateTimer::default(),
        }
    }
}
