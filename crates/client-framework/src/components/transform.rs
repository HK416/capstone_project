use std::ops;
use gmm::{
    Float3, Float4, Float4x4, 
    Matrix, Quaternion, Vector
};



/// 변환 행렬의 정보를 담고 있습니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Transform(Float4x4);

impl Transform {
    /// 새로운 변환 행렬을 생성합니다.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// 위치로부터 변환 행렬을 생성합니다.
    pub fn from_translation<T: Into<Float3>>(
        translation: T
    ) -> Self {
        let translation: Float3 = translation.into();
        Self(Matrix::from_translation(translation.into()).into())
    }

    /// 회전량과 위치로부터 변환 행렬을 생성합니다.
    pub fn from_rotation_translation<R: Into<Float4>, T: Into<Float3>>(
        rotation: R, 
        translation: T
    ) -> Self {
        let rotation: Float4 = rotation.into();
        let translation: Float3 = translation.into();
        Self(Matrix::from_rotation_translation(
            rotation.into(), 
            translation.into()
        ).into())
    }

    /// 크기, 회전량, 위치로부터 변환 행렬을 생성합니다.
    pub fn from_scale_rotation_translation<S, R, T>(
        scale: S, 
        rotation: R, 
        translation: T
    ) -> Self 
    where S: Into<Float3>, R: Into<Float4>, T: Into<Float3> {
        let scale: Float3 = scale.into();
        let rotation: Float4 = rotation.into();
        let translation: Float3 = translation.into();
        Self(Matrix::from_scale_rotation_translation(
            scale.into(), 
            rotation.into(), 
            translation.into()
        ).into())
    }

    /// 위치를 가져옵니다.
    pub fn get_translation(&self) -> Float3 {
        self.w_axis.xyz()
    }

    /// 위치를 설정합니다.
    pub fn set_translation<T: Into<Float3>>(&mut self, translation: T) {
        let translation: Float3 = translation.into();
        self.w_axis = self.w_axis
            .set_x(translation.x)
            .set_y(translation.y)
            .set_z(translation.z);
    }

    /// 오른쪽 방향의 벡터를 가져옵니다.
    pub fn get_right_vector(&self) -> Float3 {
        let x_axis: Vector = self.x_axis.into();
        let x_axis: Float3 = x_axis.vec3_normalize()
            .map(|norm| norm.into())
            .unwrap_or(Float3::X);
        return x_axis;
    }

    /// 위쪽 방향의 벡터를 가져옵니다.
    pub fn get_up_vector(&self) -> Float3 {
        let y_axis: Vector = self.y_axis.into();
        let y_axis: Float3 = y_axis.vec3_normalize()
            .map(|norm| norm.into())
            .unwrap_or(Float3::Y);
        return y_axis;
    }

    /// 앞쪽 방향의 벡터를 가져옵니다.
    pub fn get_forward_vector(&self) -> Float3 {
        let z_axis: Vector = self.z_axis.into();
        let z_axis: Float3 = z_axis.vec3_normalize()
            .map(|norm| norm.into())
            .unwrap_or(Float3::Z);
        return z_axis;
    }

    /// 회전량을 가져옵니다.
    pub fn get_rotation(&self) -> Float4 {
        let matrix: Matrix = (**self).into();
        let rotation: Quaternion = matrix.try_into()
            .unwrap_or_else(|identity| identity);
        return rotation.into();
    }

    /// 회전량을 설정합니다.
    pub fn set_rotation<R: Into<Float4>>(&mut self, rotation: R) {
        let translation = self.w_axis;
        let rotation: Float4 = rotation.into();
        let rotation: Quaternion = rotation.into();
        let rotation: Matrix = rotation.try_into()
            .unwrap_or_else(|identity| identity);
        **self = rotation.into();
        self.w_axis = translation;
    }

    /// 향하는 방향으로 주어진 `distance`만큼 이동시킵니다.
    pub fn translate_local<D: Into<Float3>>(&mut self, distance: D) {
        let distance: Float3 = distance.into();
        let right: Vector = (self.get_right_vector() * distance.x).into();
        let up: Vector = (self.get_up_vector() * distance.y).into();
        let forward: Vector = (self.get_forward_vector() * distance.z).into();
        self.translate_world(right + up + forward);
    }

    /// 축 방향으로 주어진 `distance`만큼 이동시킵니다.
    pub fn translate_world<D: Into<Float3>>(&mut self, distance: D) {
        let distance: Float3 = distance.into();
        let matrix = &mut *self;
        matrix.w_axis = matrix.w_axis
            .set_x(distance.x)
            .set_y(distance.y)
            .set_z(distance.z);
    }

    /// 주어진 회전량만큼 회전시킵니다.
    pub fn rotate<Q: Into<Float4>>(&mut self, rotation: Float4) {
        let matrix: Matrix = (**self).into();
        let rotation: Float4 = rotation.into();
        let rotation: Quaternion = rotation.into();
        let rotation: Matrix = rotation.try_into()
            .unwrap_or_else(|identity| identity);
        **self = (matrix * rotation).into();
    }

    /// x축 방향으로 `angle` 만틈 회전시킵니다.
    pub fn rotate_x_axis(&mut self, angle: f32) {
        let matrix: Matrix = (**self).into();
        let rotation: Matrix = Quaternion::from_rotation_x(angle)
            .try_into()
            .unwrap_or_else(|identity| identity);
        **self = (matrix * rotation).into();
    }

    /// y축 방향으로 `angle` 만틈 회전시킵니다.
    pub fn rotate_y_axis(&mut self, angle: f32) {
        let matrix: Matrix = (**self).into();
        let rotation: Matrix = Quaternion::from_rotation_y(angle)
            .try_into()
            .unwrap_or_else(|identity| identity);
        **self = (matrix * rotation).into();
    }

    /// z축 방향으로 `angle` 만틈 회전시킵니다.
    pub fn rotate_z_axis(&mut self, angle: f32) {
        let matrix: Matrix = (**self).into();
        let rotation: Matrix = Quaternion::from_rotation_z(angle)
            .try_into()
            .unwrap_or_else(|identity| identity);
        **self = (matrix * rotation).into();
    }
}

impl Default for Transform {
    #[inline]
    fn default() -> Self {
        Self(Float4x4::IDENTITY)
    }
}

impl ops::Deref for Transform {
    type Target = Float4x4;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Transform {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
