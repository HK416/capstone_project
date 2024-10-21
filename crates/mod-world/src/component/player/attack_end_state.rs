use std::{any::type_name, sync::Arc};

use mod_parallelism::collections::SkipMap;
use winit::{event::{Modifiers, MouseButton}, keyboard::{KeyCode, KeyLocation}};

use crate::{component::{AnimationSet, GameObject, InputController, Transform, WorldID}, render::camera::ThirdPersonCamera};

use super::{PlayerFlags, PlayerState, PlayerStateError};



/// 애플리케이션에 키보드 눌림 이벤트가 발생했을 때 호출되는 콜백 함수입니다.
pub fn on_keyboard_pressed(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    keycode: KeyCode, 
    _location: KeyLocation, 
    _modifiers: Modifiers, 
    repeat: bool
) -> Result<(), PlayerStateError> {
    if !repeat {
        // 플레이어 오브젝트를 가져옵니다.
        let mut player = match world.get_mut(player_id) {
            Some(player) => player, 
            None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
        };

        // 플레이어 컨트롤러를 가져옵니다.
        let controller = match player.get::<InputController>() {
            Some(controller) => controller.clone(), 
            None => return Err(PlayerStateError::ElementNotFound(type_name::<InputController>()))
        };

        // 플레이어 상태 플래그를 가져옵니다.
        let flags = match player.get_mut::<PlayerFlags>() {
            Some(flags) => flags, 
            None => return Err(PlayerStateError::ElementNotFound(type_name::<PlayerFlags>()))
        };

        if keycode == controller.forward {
            *flags |= PlayerFlags::Forward;
        } else if keycode == controller.backward {
            *flags |= PlayerFlags::Backward;
        } else if keycode == controller.left {
            *flags |= PlayerFlags::Left;
        } else if keycode == controller.right {
            *flags |= PlayerFlags::Right;
        }
    }

    Ok(())
}



/// 애플리케이션에 키보드 떼임 이벤트가 발생했을 때 호출되는 콜백 함수입니다.
pub fn on_keyboard_released(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    keycode: KeyCode, 
    _location: KeyLocation, 
    _modifiers: Modifiers, 
    repeat: bool
) -> Result<(), PlayerStateError> {
    if !repeat {
        // 플레이어 오브젝트를 가져옵니다.
        let mut player = match world.get_mut(player_id) {
            Some(player) => player, 
            None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
        };

        // 플레이어 컨트롤러를 가져옵니다.
        let controller = match player.get::<InputController>() {
            Some(controller) => controller.clone(), 
            None => return Err(PlayerStateError::ElementNotFound(type_name::<InputController>()))
        };

        // 플레이어 입력 방향을 가져옵니다.
        let flags = match player.get_mut::<PlayerFlags>() {
            Some(flags) => flags, 
            None => return Err(PlayerStateError::ElementNotFound(type_name::<PlayerFlags>()))
        };

        if keycode == controller.forward {
            *flags &= !PlayerFlags::Forward;
        } else if keycode == controller.backward {
            *flags &= !PlayerFlags::Backward;
        } else if keycode == controller.left {
            *flags &= !PlayerFlags::Left;
        } else if keycode == controller.right {
            *flags &= !PlayerFlags::Right;
        }
    }

    Ok(())
}



/// 애플리케이션 마우스 커서 움직임 이벤트가 발생할 때 호출되는 콜백 함수입니다.
pub fn on_cursor_moved(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    dx: f32, dy: f32
) -> Result<(), PlayerStateError> {
    // 플레이어 오브젝트의 삼인칭 카메라 요소를 가져옵니다.
    const OFFSET: f32 = 0.05;
    let mut player = world.get_mut(player_id).unwrap();
    let third_person_camera = player.get_mut::<ThirdPersonCamera>().unwrap();
    third_person_camera.polar = (third_person_camera.polar + dx.to_radians() * OFFSET) % 360f32.to_radians();
    third_person_camera.azimuthal = (third_person_camera.azimuthal + dy.to_radians() * OFFSET).clamp(-20f32.to_radians(), 45f32.to_radians());

    Ok(())
}



/// 애플리케이션 마우스 버튼 눌림 이벤트가 발생할 때 호출되는 콜백 함수입니다.
pub fn on_mouse_btn_pressed(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    _x: f32, _y: f32, 
    button: MouseButton
) -> Result<(), PlayerStateError> {
    // 플레이어 오브젝트를 가져옵니다.
    let mut player = match world.get_mut(player_id) {
        Some(player) => player, 
        None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
    };

    // 입력 제어기를 가져옵니다.
    let controller = match player.get::<InputController>() {
        Some(controller) => controller, 
        None => return Err(PlayerStateError::ElementNotFound(type_name::<InputController>()))
    };

    // 조준 버튼이 눌렸을 경우 플레이어 상태를 변경합니다.
    if controller.fire_btn == button {
        // 플래그 변수를 활성화 합니다.
        match player.get_mut::<PlayerFlags>() {
            Some(flags) => *flags |= PlayerFlags::Fire, 
            None => return Err(PlayerStateError::ElementNotFound(type_name::<PlayerFlags>()))
        };
    }

    Ok(())
}



/// 애플리케이션 마우스 버튼 떼임 이벤트가 발생할 때 호출되는 콜백 함수입니다.
pub fn on_mouse_btn_released(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    _x: f32, _y: f32, 
    button: MouseButton
) -> Result<(), PlayerStateError> {
    // 플레이어 오브젝트를 가져옵니다.
    let mut player = match world.get_mut(player_id) {
        Some(player) => player, 
        None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
    };

    // 입력 제어기를 가져옵니다.
    let controller = match player.get::<InputController>() {
        Some(controller) => controller, 
        None => return Err(PlayerStateError::ElementNotFound(type_name::<InputController>()))
    };

    // 조준 버튼이 눌렸을 경우 플레이어 상태를 변경합니다.
    if controller.fire_btn == button {
        // 플래그 변수를 비활성화 합니다.
        match player.get_mut::<PlayerFlags>() {
            Some(flags) => *flags &= !PlayerFlags::Fire, 
            None => return Err(PlayerStateError::ElementNotFound(type_name::<PlayerFlags>()))
        };
    }

    Ok(())
}



/// 애플리케이션 갱신 이벤트가 발생했을 때 호출되는 콜백 함수입니다.
pub fn on_update(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    elapsed_time_sec: f32
) -> Result<(), PlayerStateError> {
    update_animation(world, player_id, elapsed_time_sec)?;
    super::update_hierarchy(world, Transform::new(), player_id);
    Ok(())
}



/// 플레이어 애니메이션을 갱신하는 함수입니다.
pub fn update_animation(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    player_id: &WorldID, 
    elapsed_time_sec: f32
) -> Result<(), PlayerStateError> {
    // 게임 월드에서 플레이어 오브젝트를 가져옵니다.
    let mut player = match world.get_mut(player_id) {
        Some(object) => object, 
        None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
    };

    // 플래그 변수를 가져옵니다.
    let flags = match player.get::<PlayerFlags>() {
        Some(flags) => flags.clone(), 
        None => return Err(PlayerStateError::ElementNotFound(type_name::<PlayerFlags>()))
    };

    // 애니메이션 요소를 가져옵니다.
    let animation = match player.get_mut::<AnimationSet>() {
        Some(animation) => animation, 
        None => return Err(PlayerStateError::ElementNotFound(type_name::<AnimationSet>()))
    };

    // 애니메이션 타이머를 갱신합니다.
    let animation_clip = animation.clips.get(animation.index).unwrap();
    animation.timer = animation.timer + elapsed_time_sec;


    // 애니메이션 타이머가 애니메이션 길이보다 클 경우 플레이어 상태를 변경합니다.
    let diff_t = animation.timer - animation_clip.length();
    animation.timer = animation.timer.min(animation_clip.length());

    if diff_t >= 0.0 {
        if flags.contains(PlayerFlags::Fire) {
            // 애니메이션을 초기화 합니다.
            animation.index = PlayerState::AttackStart as usize;
            animation.timer = diff_t;
    
            // 플레이어 상태를 변경합니다.
            player.insert(PlayerState::AttackStart);
        } else {
            if flags.is_stopped() {
                // 애니메이션을 초기화 합니다.
                animation.index = PlayerState::Idle as usize;
                animation.timer = diff_t;
        
                // 플레이어 상태를 변경합니다.
                player.insert(PlayerState::Idle);
            } else {
                // 애니메이션을 초기화 합니다.
                animation.index = PlayerState::Moving as usize;
                animation.timer = diff_t;
        
                // 플레이어 상태를 변경합니다.
                player.insert(PlayerState::Moving);
            }
        }
        return Ok(());
    }

    // 키 프레임을 샘플링 합니다.
    let keyframe = animation_clip.sample_animation(animation.timer);

    // 최상위 뼈 노드를 가져옵니다.
    let root_id = animation_clip.root_bone_id();
    let mut root_object = match world.get_mut(root_id) {
        Some(object) => object, 
        None => return Err(PlayerStateError::ObjectNotFound(root_id.clone()))
    };

    // 최상위 뼈 노드의 변환 행렬을 설정합니다.
    root_object.set_local_transform(keyframe.root_bone());
    
    // 뼈 변환 행렬을 게임 오브젝트에 적용합니다.
    for skinning in keyframe.meshes() {
        for (index, world_id) in skinning.skinned_mesh.bones().iter().enumerate() {
            // 게임 월드에서 뼈 오브젝트를 가져옵니다.
            let mut bone_object = match world.get_mut(world_id) {
                Some(object) => object, 
                None => return Err(PlayerStateError::ObjectNotFound(world_id.clone()))
            };

            // 뼈 오브젝트의 로컬 변환 행렬을 설정합니다.
            let bone_transform = Transform(skinning.transforms[index].into());
            bone_object.set_local_transform(bone_transform);
        }
    }

    Ok(())
}
