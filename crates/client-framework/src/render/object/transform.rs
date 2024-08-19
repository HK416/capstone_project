use std::ops;



/// 오브젝트의 로컬 변환 행렬입니다.
#[derive(Debug, Clone, Copy)]
pub struct Transform(gmm::Matrix);

impl Transform {
    /// 새로운 로컬 변환 행렬을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 위치 데이터로 부터 로컬 변환 행렬을 생성합니다.
    #[inline]
    #[must_use]
    pub fn from_translation<T>(translation: T) -> Self 
    where T: Into<gmm::Vector> {
        Self(gmm::Matrix::from_translation(translation.into()))
    }

    /// 회전, 위치 데이터로부터 로컬 변환 행렬을 생성합니다.
    /// 
    /// ※ 주어진 `rotation`은 정규화된 사원수여야 합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn from_rotation_translation<R, T>(
        rotation: R, 
        translation: T
    ) -> Self 
    where T: Into<gmm::Vector>, R: Into<gmm::Quaternion> {
        Self(gmm::Matrix::from_rotation_translation(
            rotation.into(), 
            translation.into())
        )
    }

    /// 크기, 회전, 위치 데이터로부터 로컬 변환 행렬을 생성합니다.
    /// 
    /// ※ 주어진 `rotation`은 정규화된 사원수여야 합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn from_scale_rotation_translation<S, R, T>(
        scale: S, 
        rotation: R, 
        translation: T
    ) -> Self 
    where S: Into<gmm::Vector>, R: Into<gmm::Quaternion>, T: Into<gmm::Vector> {
        Self(gmm::Matrix::from_scale_rotation_translation(
            scale.into(), 
            rotation.into(), 
            translation.into())
        )
    }

    /// 주어진 거리만큼 이동합니다.
    #[inline]
    pub fn translate<D: Into<gmm::Vector>>(&mut self, distance: D) {
        self.0 = self.0 * gmm::Matrix::from_translation(distance.into());
    }

    /// 주어진 회전량 만큼 회전시킵니다.
    #[inline]
    pub fn rotate<R: Into<gmm::Quaternion>>(&mut self, rotation: R) {
        let rotation: gmm::Quaternion = rotation.into();
        let rotation: gmm::Matrix = rotation.try_into().unwrap_or_else(|identity| identity);
        self.0 = self.0 * rotation;
    }

    /// 변환 행렬의 크기, 회전, 위치를 반환합니다.
    /// 
    /// # Panics
    /// 변환 행렬의 행렬식(determinant)의 절댓값이 [`f32::EPSILON`]보다 작을 경우 [`panic!`]을 호출합니다.
    /// 
    pub fn get_scale_rotation_translation(&self) -> (gmm::Vector, gmm::Quaternion, gmm::Vector) {
        // 행렬식을 계산합니다.
        let det = {
            let det = self.0.determinant();
            let det: gmm::Float4 = det.into();
            det.x
        };
        let validate = det.abs() > f32::EPSILON;
        assert!(validate, "The absolute value of the determinant of the matrix is less than or equal `f32::EPSILON`!");
        
        let mat: gmm::Float4x4 = self.0.into();
        let v_x: gmm::Vector = mat.x_axis.into();
        let v_y: gmm::Vector = mat.y_axis.into();
        let v_z: gmm::Vector = mat.z_axis.into();
        
        // 크기를 가져옵니다.
        let det_signum = det / det.abs();
        let scale = gmm::Float3::new(
            v_x.vec3_len() * det_signum, 
            v_y.vec3_len(), 
            v_z.vec3_len()
        );

        // 회전량을 가져옵니다.
        let inv_scale = 1.0 / scale;
        let rotation: gmm::Matrix = gmm::Float3x3::from_columns(
            mat.x_axis.xyz() * inv_scale.x, 
            mat.y_axis.xyz() * inv_scale.y, 
            mat.z_axis.xyz() * inv_scale.z
        ).into();
        let rotation: gmm::Quaternion = rotation.try_into()
            .unwrap_or_else(|identity| identity);

        // 위치를 가져옵니다.
        let translation = mat.w_axis.xyz();

        return (scale.into(), rotation, translation.into())
    }

    /// 로컬 변환 행렬의 위치 정보를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_translation(&self) -> gmm::Vector {
        let transform: gmm::Float4x4 = self.0.into();
        transform.w_axis.xyz().into()
    }

    /// 로컬 변환 행렬의 오른쪽 방향 단위 벡터를 반환합니다.
    #[inline]
    #[must_use]
    pub fn get_right_vector(&self) -> gmm::Vector {
        let transform: gmm::Float4x4 = self.0.into();
        let right: gmm::Vector = transform.x_axis.xyz().into();
        right.vec3_normalize().map_or_else(|| gmm::Float3::X.into(), |v| v)
    }

    /// 로컬 변환 행렬의 위쪽 방향 단위 벡터를 반환합니다.
    #[inline]
    #[must_use]
    pub fn get_up_vector(&self) -> gmm::Vector {
        let transform: gmm::Float4x4 = self.0.into();
        let up: gmm::Vector = transform.y_axis.xyz().into();
        up.vec3_normalize().map_or_else(|| gmm::Float3::Y.into(), |v| v)
    }

    /// 로컬 변환 행렬의 앞쪽 방향 단위 벡터를 반환합니다.
    #[inline]
    #[must_use]
    pub fn get_forward_vector(&self) -> gmm::Vector {
        let transform: gmm::Float4x4 = self.0.into();
        let forward: gmm::Vector = transform.z_axis.xyz().into();
        forward.vec3_normalize().map_or_else(|| gmm::Float3::Z.into(), |v| v)
    }

    /// 로컬 변환 행렬의 회전 사원수를 반환합니다.
    #[inline]
    #[must_use]
    pub fn get_scale_rotation(&self) -> (gmm::Vector, gmm::Quaternion) {
        let (scale, rotation, _) = self.get_scale_rotation_translation();
        return (scale, rotation);
    }

    /// 로컬 변환 행렬의 크기를 설정합니다.
    pub fn set_scale<S: Into<gmm::Vector>>(&mut self, scale: S) {
        let (_, rotation, translation) = self.get_scale_rotation_translation();
        self.0 = gmm::Matrix::from_scale_rotation_translation(scale.into(), rotation, translation);
    }

    /// 로컬 변환 행렬의 회전량을 설정합니다.
    pub fn set_rotation<R: Into<gmm::Quaternion>>(&mut self, rotation: R) {
        let (scale, _, translation) = self.get_scale_rotation_translation();
        self.0 = gmm::Matrix::from_scale_rotation_translation(scale, rotation.into(), translation);
    }

    /// 로컬 변환 행렬의 위치를 설정합니다.
    pub fn set_translation<T: Into<gmm::Vector>>(&mut self, translation: T) {
        let (scale, rotation, _) = self.get_scale_rotation_translation();
        self.0 = gmm::Matrix::from_scale_rotation_translation(scale, rotation, translation.into());
    }
}

impl From<gmm::Float4x4> for Transform {
    #[inline]
    fn from(value: gmm::Float4x4) -> Self {
        Self(value.into())
    }
}

impl ops::Deref for Transform {
    type Target = gmm::Matrix;
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

impl Default for Transform {
    #[inline]
    fn default() -> Self {
        Self(gmm::Float4x4::IDENTITY.into())
    }
}



/// 오브젝트의 월드 변환 행렬입니다.
#[derive(Debug, Clone, Copy)]
pub struct WorldTransform(gmm::Matrix);

impl WorldTransform {
    /// 새로운 로컬 변환 행렬을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 위치 데이터로 부터 로컬 변환 행렬을 생성합니다.
    #[inline]
    #[must_use]
    pub fn from_translation<T>(translation: T) -> Self 
    where T: Into<gmm::Vector> {
        Self(gmm::Matrix::from_translation(translation.into()))
    }

    /// 회전, 위치 데이터로부터 로컬 변환 행렬을 생성합니다.
    /// 
    /// ※ 주어진 `rotation`은 정규화된 사원수여야 합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn from_rotation_translation<R, T>(
        rotation: R, 
        translation: T
    ) -> Self 
    where T: Into<gmm::Vector>, R: Into<gmm::Quaternion> {
        Self(gmm::Matrix::from_rotation_translation(
            rotation.into(), 
            translation.into())
        )
    }

    /// 크기, 회전, 위치 데이터로부터 로컬 변환 행렬을 생성합니다.
    /// 
    /// ※ 주어진 `rotation`은 정규화된 사원수여야 합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn from_scale_rotation_translation<S, R, T>(
        scale: S, 
        rotation: R, 
        translation: T
    ) -> Self 
    where S: Into<gmm::Vector>, R: Into<gmm::Quaternion>, T: Into<gmm::Vector> {
        Self(gmm::Matrix::from_scale_rotation_translation(
            scale.into(), 
            rotation.into(), 
            translation.into())
        )
    }

    /// 주어진 거리만큼 이동합니다.
    #[inline]
    pub fn translate<D: Into<gmm::Vector>>(&mut self, distance: D) {
        self.0 = self.0 * gmm::Matrix::from_translation(distance.into());
    }

    /// 주어진 회전량 만큼 회전시킵니다.
    #[inline]
    pub fn rotate<R: Into<gmm::Quaternion>>(&mut self, rotation: R) {
        let rotation: gmm::Quaternion = rotation.into();
        let rotation: gmm::Matrix = rotation.try_into().unwrap_or_else(|identity| identity);
        self.0 = self.0 * rotation;
    }

    /// 변환 행렬의 크기, 회전, 위치를 반환합니다.
    /// 
    /// # Panics
    /// 변환 행렬의 행렬식(determinant)의 절댓값이 [`f32::EPSILON`]보다 작을 경우 [`panic!`]을 호출합니다.
    /// 
    pub fn get_scale_rotation_translation(&self) -> (gmm::Vector, gmm::Quaternion, gmm::Vector) {
        // 행렬식을 계산합니다.
        let det = {
            let det = self.0.determinant();
            let det: gmm::Float4 = det.into();
            det.x
        };
        let validate = det.abs() > f32::EPSILON;
        assert!(validate, "The absolute value of the determinant of the matrix is less than or equal `f32::EPSILON`!");
        
        let mat: gmm::Float4x4 = self.0.into();
        let v_x: gmm::Vector = mat.x_axis.into();
        let v_y: gmm::Vector = mat.y_axis.into();
        let v_z: gmm::Vector = mat.z_axis.into();
        
        // 크기를 가져옵니다.
        let det_signum = det / det.abs();
        let scale = gmm::Float3::new(
            v_x.vec3_len() * det_signum, 
            v_y.vec3_len(), 
            v_z.vec3_len()
        );

        // 회전량을 가져옵니다.
        let inv_scale = 1.0 / scale;
        let rotation: gmm::Matrix = gmm::Float3x3::from_columns(
            mat.x_axis.xyz() * inv_scale.x, 
            mat.y_axis.xyz() * inv_scale.y, 
            mat.z_axis.xyz() * inv_scale.z
        ).into();
        let rotation: gmm::Quaternion = rotation.try_into()
            .unwrap_or_else(|identity| identity);

        // 위치를 가져옵니다.
        let translation = mat.w_axis.xyz();

        return (scale.into(), rotation, translation.into())
    }

    /// 로컬 변환 행렬의 위치 정보를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_translation(&self) -> gmm::Vector {
        let transform: gmm::Float4x4 = self.0.into();
        transform.w_axis.xyz().into()
    }

    /// 로컬 변환 행렬의 오른쪽 방향 단위 벡터를 반환합니다.
    #[inline]
    #[must_use]
    pub fn get_right_vector(&self) -> gmm::Vector {
        let transform: gmm::Float4x4 = self.0.into();
        let right: gmm::Vector = transform.x_axis.xyz().into();
        right.vec3_normalize().map_or_else(|| gmm::Float3::X.into(), |v| v)
    }

    /// 로컬 변환 행렬의 위쪽 방향 단위 벡터를 반환합니다.
    #[inline]
    #[must_use]
    pub fn get_up_vector(&self) -> gmm::Vector {
        let transform: gmm::Float4x4 = self.0.into();
        let up: gmm::Vector = transform.y_axis.xyz().into();
        up.vec3_normalize().map_or_else(|| gmm::Float3::Y.into(), |v| v)
    }

    /// 로컬 변환 행렬의 앞쪽 방향 단위 벡터를 반환합니다.
    #[inline]
    #[must_use]
    pub fn get_forward_vector(&self) -> gmm::Vector {
        let transform: gmm::Float4x4 = self.0.into();
        let forward: gmm::Vector = transform.z_axis.xyz().into();
        forward.vec3_normalize().map_or_else(|| gmm::Float3::Z.into(), |v| v)
    }

    /// 로컬 변환 행렬의 회전 사원수를 반환합니다.
    #[inline]
    #[must_use]
    pub fn get_scale_rotation(&self) -> (gmm::Vector, gmm::Quaternion) {
        let (scale, rotation, _) = self.get_scale_rotation_translation();
        return (scale, rotation);
    }

    /// 로컬 변환 행렬의 크기를 설정합니다.
    pub fn set_scale<S: Into<gmm::Vector>>(&mut self, scale: S) {
        let (_, rotation, translation) = self.get_scale_rotation_translation();
        self.0 = gmm::Matrix::from_scale_rotation_translation(scale.into(), rotation, translation);
    }

    /// 로컬 변환 행렬의 회전량을 설정합니다.
    pub fn set_rotation<R: Into<gmm::Quaternion>>(&mut self, rotation: R) {
        let (scale, _, translation) = self.get_scale_rotation_translation();
        self.0 = gmm::Matrix::from_scale_rotation_translation(scale, rotation.into(), translation);
    }

    /// 로컬 변환 행렬의 위치를 설정합니다.
    pub fn set_translation<T: Into<gmm::Vector>>(&mut self, translation: T) {
        let (scale, rotation, _) = self.get_scale_rotation_translation();
        self.0 = gmm::Matrix::from_scale_rotation_translation(scale, rotation, translation.into());
    }
}

impl From<gmm::Matrix> for WorldTransform {
    #[inline]
    fn from(value: gmm::Matrix) -> Self {
        Self(value)
    }
}

impl Into<gmm::Float4x4> for WorldTransform {
    #[inline]
    fn into(self) -> gmm::Float4x4 {
        self.0.into()
    }
}

impl ops::Deref for WorldTransform {
    type Target = gmm::Matrix;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for WorldTransform {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for WorldTransform {
    #[inline]
    fn default() -> Self {
        Self(gmm::Float4x4::IDENTITY.into())
    }
}
