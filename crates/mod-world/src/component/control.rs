use winit::{event::MouseButton, keyboard::KeyCode};



/// 사용자 지정 입력 제어기입니다.
#[derive(Debug, Clone)]
pub struct InputController {
    pub forward: KeyCode, 
    pub backward: KeyCode, 
    pub left: KeyCode, 
    pub right: KeyCode, 
    pub aim_btn: MouseButton, 
    pub fire_btn: MouseButton, 
}

impl Default for InputController {
    #[inline]
    fn default() -> Self {
        Self { 
            forward: KeyCode::KeyW, 
            backward: KeyCode::KeyS, 
            left: KeyCode::KeyA, 
            right: KeyCode::KeyD, 
            aim_btn: MouseButton::Right, 
            fire_btn: MouseButton::Left, 
        }
    }
}
