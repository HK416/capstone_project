/// ## Tag
/// 엔터티가 카메라임을 식별하는 태그입니다.
pub struct CameraTag;

/// ## Camera Behavior State
#[derive(Debug, Clone, Copy)]
pub enum CameraBehaviorState {
    Idle,
    Aimming,
    EnterAimming(f32),
    ExitAimming(f32),
}

/// ## View Frustum
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub near: f32,
    pub far: f32,
}

impl Frustum {
    /// 원근 투영 변환 행렬로부터 `Frustum`을 생성합니다.
    ///
    /// # Note
    /// 잘못된 원근 투영 변환 행렬이 주어졌을 경우 `Frustum`을 계산하는 도중 0 나누기 오류가 발생할 수 있습니다.
    ///
    pub fn from_perspective(mat: &glam::Mat4) -> Self {
        let m00 = mat.x_axis.x;
        let m02 = mat.z_axis.x;
        let m11 = mat.y_axis.y;
        let m12 = mat.z_axis.y;
        let m22 = mat.z_axis.z;
        let m23 = mat.w_axis.z;

        let near = m23 / (m22 - 1.0);
        let far = m23 / (m22 + 1.0);
        let left = -near * (1.0 + m02) / m00;
        let right = near * (1.0 - m02) / m00;
        let bottom = -near * (1.0 + m12) / m11;
        let top = near * (1.0 - m12) / m11;

        Self {
            top,
            left,
            bottom,
            right,
            near,
            far,
        }
    }
}

/// ## Projection Transform Matrix
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection(pub glam::Mat4);

impl Projection {
    /// 새로운 원근 투영 변환 행렬을 생성합니다.
    pub fn perspective(fov_y_radians: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Self {
        Self(glam::Mat4::perspective_lh(
            fov_y_radians,
            aspect_ratio,
            z_near,
            z_far,
        ))
    }
}
