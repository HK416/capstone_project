use std::{any::TypeId, sync::Arc};

use mod_parallelism::collections::SkipMap;
use winit::{event::{Modifiers, MouseButton}, keyboard::{KeyCode, KeyLocation}};

use crate::component::{AnimationSet, Direction, GameObject, InputController, ThirdPersonCamera, Transform, WorldID};

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
            None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<InputController>()))
        };

        // 플레이어 입력 방향을 가져옵니다.
        let direction = match player.get_mut::<Direction>() {
            Some(direction) => direction, 
            None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<Direction>()))
        };


        if keycode == controller.forward {
            *direction |= Direction::Forward;
        } else if keycode == controller.backward {
            *direction |= Direction::Backward;
        } else if keycode == controller.left {
            *direction |= Direction::Left;
        } else if keycode == controller.right {
            *direction |= Direction::Right;
        }

        if !direction.is_stopped() {
            // 플레이어 오브젝트를 가져옵니다.
            let mut player = match world.get_mut(player_id) {
                Some(object) => object, 
                None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
            };

            // 플레이어 애니메이션을 가져옵니다.
            let animation = match player.get_mut::<AnimationSet>() {
                Some(animation) => animation, 
                None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<AnimationSet>()))
            };

            // 애니메이션을 초기화 합니다.
            animation.index = PlayerState::Moving as usize;
            animation.timer = 0.0;

            // 플레이어 상태를 변경합니다.
            player.insert(PlayerState::Moving);
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
            None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<InputController>()))
        };

        // 플레이어 입력 방향을 가져옵니다.
        let direction = match player.get_mut::<Direction>() {
            Some(direction) => direction, 
            None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<Direction>()))
        };


        if keycode == controller.forward {
            *direction &= !Direction::Forward;
        } else if keycode == controller.backward {
            *direction &= !Direction::Backward;
        } else if keycode == controller.left {
            *direction &= !Direction::Left;
        } else if keycode == controller.right {
            *direction &= !Direction::Right;
        }

        if !direction.is_stopped() {
            // 플레이어 오브젝트를 가져옵니다.
            let mut player = match world.get_mut(player_id) {
                Some(object) => object, 
                None => return Err(PlayerStateError::ObjectNotFound(player_id.clone()))
            };

            // 플레이어 애니메이션을 가져옵니다.
            let animation = match player.get_mut::<AnimationSet>() {
                Some(animation) => animation, 
                None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<AnimationSet>()))
            };

            // 애니메이션을 초기화 합니다.
            animation.index = PlayerState::Moving as usize;
            animation.timer = 0.0;

            // 플레이어 상태를 변경합니다.
            player.insert(PlayerState::Moving);
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
    // 카메라 오브젝트의 삼인칭 카메라 요소를 가져옵니다.
    let mut player = world.get_mut(player_id).unwrap();
    let third_person = player.get_mut::<ThirdPersonCamera>().unwrap();
    third_person.polar = (third_person.polar + dx.to_radians()) % 360f32.to_radians();
    third_person.azimuthal = (third_person.azimuthal + dy.to_radians()).clamp(-10f32.to_radians(), 30f32.to_radians());

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
        None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<InputController>()))
    };

    // 조준 버튼이 눌렸을 경우 플레이어 상태를 변경합니다.
    if controller.fire_btn == button {
        // 플래그 변수를 활성화 합니다.
        match player.get_mut::<PlayerFlags>() {
            Some(flags) => *flags |= PlayerFlags::Fire, 
            None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<PlayerFlags>()))
        };

        // 카메라 오브젝트의 식별자를 가져옵니다.
        let camera_id = match player.get::<ThirdPersonCamera>() {
            Some(third_person_camera) => third_person_camera.target.clone(), 
            None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<ThirdPersonCamera>()))
        };

        // 카메라 오브젝트의 월드 변환 행렬을 가져옵니다.
        let camera_transform = match world.get(&camera_id) {
            Some(camera) => camera.get_world_transform().clone(), 
            None => return Err(PlayerStateError::ObjectNotFound(camera_id))
        };

        // 플레이어 방향을 카메라의 방향과 일치시킵니다.
        let z_axis = camera_transform.get_look_vector();
        let y_axis = gmm::Vector::Y;
        let x_axis = y_axis.vec3_cross(z_axis);
        let z_axis = x_axis.vec3_cross(y_axis);
        let rotation = gmm::Quaternion::from_rotation_axes(x_axis, y_axis, z_axis);
        let mut transform = player.get_local_transform().clone();
        transform.set_rotation(rotation);
        player.set_local_transform(transform);


        // 플레이어 애니메이션을 가져옵니다.
        let animation = match player.get_mut::<AnimationSet>() {
            Some(animation) => animation, 
            None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<AnimationSet>()))
        };

        // 애니메이션을 초기화 합니다.
        animation.index = PlayerState::AttackStart as usize;
        animation.timer = 0.0;

        // 플레이어 상태를 변경합니다.
        player.insert(PlayerState::AttackStart);
    }

    Ok(())
}



/// 애플리케이션 마우스 버튼 떼임 이벤트가 발생할 때 호출되는 콜백 함수입니다.
pub fn on_mouse_btn_released(
    _world: &Arc<SkipMap<WorldID, GameObject>>, 
    _player_id: &WorldID, 
    _x: f32, _y: f32, 
    _button: MouseButton
) -> Result<(), PlayerStateError> {
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

    // 애니메이션 요소를 가져옵니다.
    let animation = match player.get_mut::<AnimationSet>() {
        Some(animation) => animation, 
        None => return Err(PlayerStateError::ElementNotFound(TypeId::of::<AnimationSet>()))
    };

    // 애니메이션 타이머를 갱신합니다.
    let animation_clip = animation.clips.get(animation.index).unwrap();
    animation.timer = (animation.timer + elapsed_time_sec) % animation_clip.length();

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
