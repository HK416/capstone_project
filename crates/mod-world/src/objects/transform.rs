/// 변환 행렬 `trait`
pub trait Transform {
    /// 변환 행렬의 스케일을 가져옵니다.
    fn get_scale(&self) -> gmm::Vector;

    /// 변환 행렬의 스케일을 설정합니다.
    fn set_scale(&mut self, scale: impl Into<gmm::Vector>);

    /// 변환 행렬의 회전 쿼터니언을 설정합니다.
    fn get_rotation(&self) -> gmm::Quaternion;

    /// 변한 행렬의 회전 쿼터니언을 설정합니다.
    fn set_rotation(&mut self, rotation: impl Into<gmm::Quaternion>);

    /// 변환 행렬의 위치를 가져옵니다.
    fn get_translation(&self) -> gmm::Vector;

    /// 변환 행렬의 위치를 설정합니다.
    fn set_translation(&mut self, translation: impl Into<gmm::Vector>);

    /// 오른쪽을 향하는 벡터를 가져옵니다.
    fn get_right_vector(&self) -> gmm::Vector;

    /// 위쪽을 향하는 벡터를 가져옵니다.
    fn get_up_vector(&self) -> gmm::Vector;

    /// 앞쪽을 향하는 벡터를 가져옵니다.
    fn get_look_vector(&self) -> gmm::Vector;

    /// 주어진 거리만큼 변환 행렬의 위치를 이동합니다.
    fn translate(&mut self, distance: impl Into<gmm::Vector>);

    /// 주어진 회전 쿼터니언 만큼 변환 행렬의 회전 쿼터니언을 회전합니다.
    fn rotate(&mut self, rotate: impl Into<gmm::Quaternion>);
}





impl Transform for gmm::Matrix {
    #[must_use]
    fn get_scale(&self) -> gmm::Vector {
        let sx = self.get_x_axis().vec3_len();
        let sy = self.get_y_axis().vec3_len();
        let sz = self.get_z_axis().vec3_len();
        gmm::Vector::new(sx, sy, sz, 0.0)
    }

    fn set_scale(&mut self, scale: impl Into<gmm::Vector>) {
        let scale: gmm::Vector = scale.into();
        let rotation = self.get_rotation();
        let translation = self.get_translation();
        *self = gmm::Matrix::from_scale_rotation_translation(
            scale, 
            rotation, 
            translation
        )
    }

    #[must_use]
    fn get_rotation(&self) -> gmm::Quaternion {
        let x_axis = self.get_x_axis().vec3_normalize();
        let y_axis = self.get_y_axis().vec3_normalize();
        let z_axis = self.get_z_axis().vec3_normalize();
        gmm::Quaternion::from_rotation_axes(x_axis, y_axis, z_axis)
    }

    fn set_rotation(&mut self, rotation: impl Into<gmm::Quaternion>) {
        let mut rotation: gmm::Quaternion = rotation.into();
        rotation = rotation.normalize();
        let scale = self.get_scale();
        let translation = self.get_translation();
        *self = gmm::Matrix::from_scale_rotation_translation(
            scale, 
            rotation, 
            translation
        )
    }

    #[must_use]
    fn get_translation(&self) -> gmm::Vector {
        let mut translation = self.get_w_axis().clone();
        translation.set_w(1.0);
        translation
    }

    fn set_translation(&mut self, translation: impl Into<gmm::Vector>) {
        let mut translation: gmm::Vector = translation.into();
        translation.set_w(1.0);
        self.set_w_axis(translation);
    }


    #[inline]
    #[must_use]
    fn get_right_vector(&self) -> gmm::Vector {
        self.get_x_axis().vec3_normalize()
    }

    #[inline]
    #[must_use]
    fn get_up_vector(&self) -> gmm::Vector {
        self.get_y_axis().vec3_normalize()
    }

    #[inline]
    #[must_use]
    fn get_look_vector(&self) -> gmm::Vector {
        self.get_z_axis().vec3_normalize()
    }


    fn translate(&mut self, distance: impl Into<gmm::Vector>) {
        let mut distance: gmm::Vector = distance.into();
        distance.set_w(0.0);

        let translation = self.get_translation();
        self.set_w_axis(translation + distance);
    }

    fn rotate(&mut self, rotate: impl Into<gmm::Quaternion>) {
        let rotate: gmm::Quaternion = rotate.into();
        let rotate = rotate.normalize();
        let rotate = rotate.into_matrix();
        *self = *self * rotate;
    }
}
