use std::{any::TypeId, sync::Arc};

use mod_parallelism::collections::SkipMap;
use winit::{event::{Modifiers, MouseButton}, keyboard::{KeyCode, KeyLocation}};

use super::{GameObject, Transform, WorldID};



/// 플레이어 상태 목록입니다.
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlayerState {
    Idle = 0, 
    Moving = 1, 
    MoveToEnd = 2, 
    AttackStart = 3, 
    Attacking = 4, 
    AttackEnd = 5, 
}

impl Default for PlayerState {
    #[inline]
    fn default() -> Self {
        Self::Idle
    }
}



/// 플레이어 상태 오류 목록입니다.
#[derive(Debug, thiserror::Error)]
pub enum PlayerStateError {
    /// 게임 월드에서 게임 오브젝트를 찾을 수 없는 경우 발생하는 오류입니다.
    #[error("GameObject not found in game world! ({0:?})")]
    ObjectNotFound(WorldID), 

    /// 게임 오브젝트에서 요소를 찾을 수 없는 경우 발생하는 오류입니다.
    #[error("Could not find ({0:?}) element on the game object!")]
    ElementNotFound(TypeId), 
}



/// 플레이어 오브젝트의 계층 구조를 갱신합니다.
fn update_hierarchy(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    parent: Transform, 
    id: &WorldID
) {
    let mut object = world.get_mut(&id).unwrap();
    let local_transform = object.get_local_transform().clone();
    let world_transform = parent * local_transform;
    object.set_world_transform(world_transform);

    let sibling_id = object.get_sibling().cloned();
    let child_id = object.get_child().cloned();

    if let Some(sibling_id) = sibling_id {
        update_hierarchy(world, parent, &sibling_id);
    }

    if let Some(child_id) = child_id {
        update_hierarchy(world, world_transform, &child_id);
    }
}



/// 플레이어 오브젝트의 키보드 눌림 이벤트를 처리하는 함수입니다.
pub fn player_keyboard_pressed(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    keycode: KeyCode, 
    location: KeyLocation, 
    modifiers: Modifiers, 
    repeat: bool
) -> Result<(), PlayerStateError> {
    type CallbackFn = fn(
        &Arc<SkipMap<WorldID, GameObject>>, 
        &WorldID, 
        KeyCode, 
        KeyLocation, 
        Modifiers, 
        bool
    ) -> Result<(), PlayerStateError>;
    const CALLBACK_FN: [CallbackFn; 6] = [
        idle_state::on_keyboard_pressed as CallbackFn, 
        moving_state::on_keyboard_pressed as CallbackFn, 
        move_to_end_state::on_keyboard_pressed as CallbackFn, 
        attack_start_state::on_keyboard_pressed as CallbackFn, 
        attacking_state::on_keyboard_pressed as CallbackFn, 
        attack_end_state::on_keyboard_pressed as CallbackFn, 
    ];

    // 플레이어 오브젝트를 가져옵니다.
    let player = match world.get(player_id) {
        Some(object) => object, 
        None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
    };

    // 플레이어 상태를 가져옵니다.
    let state = match player.get::<PlayerState>() {
        Some(state) => state.clone(), 
        None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<PlayerState>()))
    };

    CALLBACK_FN[state as usize](
        world, 
        player_id, 
        keycode, 
        location, 
        modifiers, 
        repeat
    )
}



/// 플레이어 오브젝트의 키보드 떼임 이벤트를 처리하는 함수입니다.
pub fn player_keyboard_released(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    keycode: KeyCode, 
    location: KeyLocation, 
    modifiers: Modifiers, 
    repeat: bool
) -> Result<(), PlayerStateError> {
    type CallbackFn = fn(
        &Arc<SkipMap<WorldID, GameObject>>, 
        &WorldID, 
        KeyCode, 
        KeyLocation, 
        Modifiers, 
        bool
    ) -> Result<(), PlayerStateError>;
    const CALLBACK_FN: [CallbackFn; 6] = [
        idle_state::on_keyboard_released as CallbackFn, 
        moving_state::on_keyboard_released as CallbackFn, 
        move_to_end_state::on_keyboard_released as CallbackFn, 
        attack_start_state::on_keyboard_released as CallbackFn, 
        attacking_state::on_keyboard_released as CallbackFn, 
        attack_end_state::on_keyboard_released as CallbackFn, 
    ];

    // 플레이어 오브젝트를 가져옵니다.
    let player = match world.get(player_id) {
        Some(object) => object, 
        None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
    };

    // 플레이어 상태를 가져옵니다.
    let state = match player.get::<PlayerState>() {
        Some(state) => state.clone(), 
        None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<PlayerState>()))
    };

    CALLBACK_FN[state as usize](
        world, 
        player_id, 
        keycode, 
        location, 
        modifiers, 
        repeat
    )
}



/// 애플리케이션 마우스 커서 움직임 이벤트가 발생할 때 호출되는 콜백 함수입니다.
pub fn player_cursor_moved(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    dx: f32, dy: f32
) -> Result<(), PlayerStateError> {
    type CallbackFn = fn(
        &Arc<SkipMap<WorldID, GameObject>>, 
        &WorldID, 
        f32, f32
    ) -> Result<(), PlayerStateError>;
    const CALLBACK_FN: [CallbackFn; 6] = [
        idle_state::on_cursor_moved as CallbackFn, 
        moving_state::on_cursor_moved as CallbackFn, 
        move_to_end_state::on_cursor_moved as CallbackFn, 
        attack_start_state::on_cursor_moved as CallbackFn, 
        attacking_state::on_cursor_moved as CallbackFn, 
        attack_end_state::on_cursor_moved as CallbackFn, 
    ];

    // 플레이어 오브젝트를 가져옵니다.
    let player = match world.get(player_id) {
        Some(object) => object, 
        None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
    };

    // 플레이어 상태를 가져옵니다.
    let state = match player.get::<PlayerState>() {
        Some(state) => state.clone(), 
        None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<PlayerState>()))
    };

    CALLBACK_FN[state as usize](world, player_id, dx, dy)
}



/// 애플리케이션 마우스 버튼 눌림 이벤트가 발생할 때 호출되는 콜백 함수입니다.
pub fn player_mouse_btn_pressed(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    x: f32, y: f32, 
    button: MouseButton
) -> Result<(), PlayerStateError> {
    type CallbackFn = fn(
        &Arc<SkipMap<WorldID, GameObject>>, 
        &WorldID, 
        f32, f32, 
        MouseButton
    ) -> Result<(), PlayerStateError>;
    const CALLBACK_FN: [CallbackFn; 6] = [
        idle_state::on_mouse_btn_pressed as CallbackFn, 
        moving_state::on_mouse_btn_pressed as CallbackFn, 
        move_to_end_state::on_mouse_btn_pressed as CallbackFn, 
        attack_start_state::on_mouse_btn_pressed as CallbackFn, 
        attacking_state::on_mouse_btn_pressed as CallbackFn, 
        attack_end_state::on_mouse_btn_pressed as CallbackFn, 
    ];

    // 플레이어 오브젝트를 가져옵니다.
    let player = match world.get(player_id) {
        Some(object) => object, 
        None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
    };

    // 플레이어 상태를 가져옵니다.
    let state = match player.get::<PlayerState>() {
        Some(state) => state.clone(), 
        None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<PlayerState>()))
    };

    CALLBACK_FN[state as usize](
        world, 
        player_id, 
        x, 
        y, 
        button
    )
}



/// 애플리케이션 마우스 버튼 떼임 이벤트가 발생할 때 호출되는 콜백 함수입니다.
pub fn player_mouse_btn_released(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    x: f32, y: f32, 
    button: MouseButton
) -> Result<(), PlayerStateError> {
    type CallbackFn = fn(
        &Arc<SkipMap<WorldID, GameObject>>, 
        &WorldID, 
        f32, f32, 
        MouseButton
    ) -> Result<(), PlayerStateError>;
    const CALLBACK_FN: [CallbackFn; 6] = [
        idle_state::on_mouse_btn_released as CallbackFn, 
        moving_state::on_mouse_btn_released as CallbackFn, 
        move_to_end_state::on_mouse_btn_released as CallbackFn, 
        attack_start_state::on_mouse_btn_released as CallbackFn, 
        attacking_state::on_mouse_btn_released as CallbackFn, 
        attack_end_state::on_mouse_btn_released as CallbackFn, 
    ];

    // 플레이어 오브젝트를 가져옵니다.
    let player = match world.get(player_id) {
        Some(object) => object, 
        None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
    };

    // 플레이어 상태를 가져옵니다.
    let state = match player.get::<PlayerState>() {
        Some(state) => state.clone(), 
        None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<PlayerState>()))
    };

    CALLBACK_FN[state as usize](
        world, 
        player_id, 
        x, 
        y, 
        button
    )
}




/// 플레이어 오브젝트를 갱신하는 함수입니다.
pub fn player_update(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    elapsed_time_sec: f32
) -> Result<(), PlayerStateError> {
    type CallbackFn = fn(
        &Arc<SkipMap<WorldID, GameObject>>, 
        &WorldID, 
        f32
    ) -> Result<(), PlayerStateError>;
    const CALLBACK_FN: [CallbackFn; 6] = [
        idle_state::on_update as CallbackFn, 
        moving_state::on_update as CallbackFn, 
        move_to_end_state::on_update as CallbackFn, 
        attack_start_state::on_update as CallbackFn, 
        attacking_state::on_update as CallbackFn, 
        attack_end_state::on_update as CallbackFn, 
    ];

    // 플레이어 오브젝트를 가져옵니다.
    let player = match world.get(player_id) {
        Some(object) => object, 
        None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
    };

    // 플레이어 상태를 가져옵니다.
    let state = match player.get::<PlayerState>() {
        Some(state) => state.clone(), 
        None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<PlayerState>()))
    };

    CALLBACK_FN[state as usize](
        world, 
        player_id, 
        elapsed_time_sec
    )
}



mod flag;
mod idle_state;
mod moving_state;
mod move_to_end_state;
mod attack_start_state;
mod attacking_state;
mod attack_end_state;

pub use self::flag::*;
