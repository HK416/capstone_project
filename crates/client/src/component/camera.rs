/// ## Camera Tag
/// `Entity`가 카메라임을 식별하는 태그입니다.
pub struct CameraTag;

/// ## Camera Behavior State
#[derive(Debug, Clone, Copy)]
pub enum CameraBehaviorState {
    Idle,
    Aimming,
    EnterAimming(f32),
    ExitAimming(f32),
}

/// ## Projection Transform Matrix
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection(pub glam::Mat4);
