use std::ops;



/// 변환 행렬 데이터입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Transform(pub gmm::Matrix);

impl Transform {
    /// 새로운 변환 행렬을 생성합니다.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(gmm::Matrix::IDENTITY)
    }


    /// 변환 행렬의 스케일을 가져옵니다.
    #[must_use]
    pub fn get_scale(&self) -> gmm::Vector {
        let sx = self.0.get_x_axis().vec3_len();
        let sy = self.0.get_y_axis().vec3_len();
        let sz = self.0.get_z_axis().vec3_len();
        gmm::Vector::new(sx, sy, sz, 0.0)
    }

    /// 변환 행렬의 스케일을 설정합니다.
    pub fn set_scale(&mut self, scale: impl Into<gmm::Vector>) {
        let scale: gmm::Vector = scale.into();
        let rotation = self.get_rotation();
        let translation = self.get_translation();
        self.0 = gmm::Matrix::from_scale_rotation_translation(scale, rotation, translation);
    }

    /// 변환 행렬의 회전 쿼터니언을 가져옵니다.
    #[must_use]
    pub fn get_rotation(&self) -> gmm::Quaternion {
        let x_axis = self.0.get_x_axis().vec3_normalize();
        let y_axis = self.0.get_y_axis().vec3_normalize();
        let z_axis = self.0.get_z_axis().vec3_normalize();
        gmm::Quaternion::from_rotation_axes(x_axis, y_axis, z_axis)
    }

    /// 변환 행렬의 회전 쿼터니언을 설정합니다.
    pub fn set_rotation(&mut self, rotation: impl Into<gmm::Quaternion>) {
        let rotation: gmm::Quaternion = rotation.into();
        let rotation = rotation.normalize();
        let scale = self.get_scale();
        let translation = self.get_translation();
        self.0 = gmm::Matrix::from_scale_rotation_translation(scale, rotation, translation);
    }

    /// 변환 행렬의 위치를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_translation(&self) -> gmm::Vector {
        self.0.get_w_axis().clone()
    }

    /// 변환 행렬의 위치를 설정합니다.
    #[allow(unused_must_use)] // gmm 라이브러리 문제 (추후 수정)
    pub fn set_translation(&mut self, translation: impl Into<gmm::Vector>) {
        let mut translation: gmm::Vector = translation.into();
        translation.set_w(1.0);
        self.0.set_w_axis(translation);
    }


    /// 주어진 거리만큼 변환 행렬의 위치를 이동합니다.
    #[allow(unused_must_use)] // gmm 라이브러리 문제 (추후 수정)
    pub fn translate(&mut self, distance: impl Into<gmm::Vector>) {
        let mut distance: gmm::Vector = distance.into();
        distance.set_w(0.0);
        
        let translation = self.get_translation();
        self.0.set_w_axis(translation + distance);
    }

    /// 주어진 회전 쿼터니언 만큼 변환 행렬의 회전 쿼터니언을 회전시킵니다.
    pub fn rotate(&mut self, rotate: impl Into<gmm::Quaternion>) {
        let rotate: gmm::Quaternion = rotate.into();
        let rotate = rotate.normalize();
        let rotate = rotate.into_matrix();
        self.0 = self.0 * rotate;
    }
}

impl From<gmm::Matrix> for Transform {
    #[inline]
    fn from(value: gmm::Matrix) -> Self {
        Transform(value)
    }
}

impl Into<gmm::Matrix> for Transform {
    #[inline]
    fn into(self) -> gmm::Matrix {
        self.0
    }
}

impl Default for Transform {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ops::Mul<f32> for Transform {
    type Output = Transform;
    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        Transform(self.0 * rhs)
    }
}

impl ops::Mul<Transform> for f32 {
    type Output = Transform;
    #[inline]
    fn mul(self, rhs: Transform) -> Self::Output {
        Transform(self * rhs.0)
    }
}

impl ops::MulAssign<f32> for Transform {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs
    }
}

impl ops::Mul<Self> for Transform {
    type Output = Transform;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Transform(self.0 * rhs.0)
    }
}

impl ops::MulAssign<Self> for Transform {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs
    }
}
