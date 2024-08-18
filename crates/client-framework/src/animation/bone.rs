/// 뼈 변환 데이터입니다.
#[derive(Debug, Clone)]
pub struct BoneTransform {
    pub scale: gmm::Float3, 
    pub rotation: gmm::Float4, 
    pub translation: gmm::Float3, 
}

impl BoneTransform {
    /// 뼈 변환 데이터로부터 행렬을 생성하여 반환합니다.
    #[inline]
    #[must_use]
    pub fn as_matrix(&self) -> gmm::Matrix {
        gmm::Matrix::from_scale_rotation_translation(
            self.scale.into(), 
            self.rotation.into(), 
            self.translation.into()
        )
    }

    /// 두 뼈 변환 데이터를 선형보간합니다.
    /// `t`의 값은 0.0 ~ 1.0사이의 값이며 1.0에 가까울 수록 `other`에 가까워집니다.
    /// 
    pub fn linear_interpolation(&self, other: &Self, t: f32) -> BoneTransform {
        let t = t.clamp(0.0, 1.0);
        BoneTransform { 
            scale: (1.0 - t) * self.scale + t * other.scale,  
            rotation: (1.0 - t) * self.rotation + t * other.rotation, 
            translation: (1.0 - t) * self.translation + t * other.translation 
        }
    }
}
