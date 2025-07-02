//! 캐릭터 이동 속도와 관련된 코드를 관리합니다.
//!

use crate::components::{
    ActionState, CharacterAttributes, InputStateTimer, MovementState, MovementStateTimer,
    MovingDirection,
};

/// 플레이어의 월드 공간 속도 데이터
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Velocity(pub glam::Vec3A);

impl Velocity {
    /// 새로운 속도 데이터를 생성합니다.
    pub const fn new() -> Self {
        Self(glam::Vec3A::ZERO)
    }

    pub fn update(
        &mut self,
        direction: &MovingDirection,
        input_timer: InputStateTimer,
        action_state: ActionState,
        movement_state: MovementState,
        movement_state_timer: MovementStateTimer,
        character_attributes: &CharacterAttributes,
    ) {
        match action_state {
            ActionState::Idle => match movement_state {
                MovementState::Idle => self.update_when_idle(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                MovementState::Moving => self.update_when_moving(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                MovementState::MoveToEnd => self.update_when_move_to_end(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                _ => {}
            },
            ActionState::AimAt => match movement_state {
                MovementState::Idle => self.update_when_idle(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                MovementState::Moving => self.update_when_move_to_aim_move(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                MovementState::MoveToEnd => self.update_when_move_to_end(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                _ => {}
            },
            ActionState::AimOff => match movement_state {
                MovementState::Idle => self.update_when_idle(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                MovementState::Moving => self.update_when_aim_move_to_move(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                MovementState::MoveToEnd => self.update_when_move_to_end(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                _ => {}
            },
            ActionState::Aiming
            | ActionState::Attack
            | ActionState::Reload
            | ActionState::Skill => match movement_state {
                MovementState::Idle => self.update_when_idle(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                MovementState::Moving => self.update_when_walking(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                MovementState::MoveToEnd => self.update_when_move_to_end(
                    direction,
                    input_timer,
                    movement_state_timer,
                    character_attributes,
                ),
                _ => {}
            },
            ActionState::Death
            | ActionState::Callsign
            | ActionState::VictoryStart
            | ActionState::VictoryEnd => {}
        }
    }

    /// [`ActionState::Idle`]일 때 플레이어 속도를 갱신합니다.
    fn update_when_idle(
        &mut self,
        direction: &MovingDirection,
        input_timer: InputStateTimer,
        _movement_state_timer: MovementStateTimer,
        character_attributes: &CharacterAttributes,
    ) {
        let s = input_timer.perentage();
        let delta = s * s * (3.0 - 2.0 * s);
        let speed = 0.5 * character_attributes.speed * delta;
        self.0.x = direction.0.x * speed;
        self.0.z = direction.0.z * speed;
    }

    /// [`ActionState::Idle`]이 아니고, [`MovementState::Moving`]일 떄 때 플레이어 속도를 갱신합니다.
    fn update_when_walking(
        &mut self,
        direction: &MovingDirection,
        input_timer: InputStateTimer,
        _movement_state_timer: MovementStateTimer,
        character_attributes: &CharacterAttributes,
    ) {
        let s = input_timer.perentage();
        let delta = s * s * (3.0 - 2.0 * s);
        let speed = 0.5 * character_attributes.speed * delta;
        self.0.x = direction.0.x * speed;
        self.0.z = direction.0.z * speed;
    }

    /// [`ActionState::Idle`]이고, [`MovementState::Moving`]일 떄 때 플레이어 속도를 갱신합니다.
    fn update_when_moving(
        &mut self,
        direction: &MovingDirection,
        input_timer: InputStateTimer,
        _movement_state_timer: MovementStateTimer,
        character_attributes: &CharacterAttributes,
    ) {
        let s = input_timer.perentage();
        let delta = s * s * (3.0 - 2.0 * s);
        let speed = character_attributes.speed * delta;
        self.0.x = direction.0.x * speed;
        self.0.z = direction.0.z * speed;
    }

    /// [`MovementState::MoveToEnd`]일 떄 때 플레이어 속도를 갱신합니다.
    fn update_when_move_to_end(
        &mut self,
        direction: &MovingDirection,
        input_timer: InputStateTimer,
        _movement_state_timer: MovementStateTimer,
        character_attributes: &CharacterAttributes,
    ) {
        let s = input_timer.perentage();
        let delta = s * s * (3.0 - 2.0 * s);
        let speed = character_attributes.speed * delta;
        self.0.x = direction.0.x * speed;
        self.0.z = direction.0.z * speed;
    }

    /// [`ActionState::AimAt`]이고, [`MovementState::Moving`]일 떄 때 플레이어 속도를 갱신합니다.
    fn update_when_move_to_aim_move(
        &mut self,
        direction: &MovingDirection,
        _input_timer: InputStateTimer,
        movement_state_timer: MovementStateTimer,
        character_attributes: &CharacterAttributes,
    ) {
        let duration = character_attributes.normal_attack_start_duration;
        let s = movement_state_timer.0 as f32 / duration as f32;
        let delta = 0.5 + 0.5 * s;
        let speed = character_attributes.speed * delta;
        self.0.x = direction.0.x * speed;
        self.0.z = direction.0.z * speed;
    }

    /// [`ActionState::AimOff`]이고, [`MovementState::Moving`]일 떄 때 플레이어 속도를 갱신합니다.
    fn update_when_aim_move_to_move(
        &mut self,
        direction: &MovingDirection,
        _input_timer: InputStateTimer,
        movement_state_timer: MovementStateTimer,
        character_attributes: &CharacterAttributes,
    ) {
        let duration = character_attributes.normal_attack_end_duration;
        let s = movement_state_timer.0 as f32 / duration as f32;
        let delta = 0.5 + 0.5 * s;
        let speed = character_attributes.speed * delta;
        self.0.x = direction.0.x * speed;
        self.0.z = direction.0.z * speed;
    }
}
