use std::sync::Arc;

use crate::skin::Skin;



/// 키 프레임의 뼈 데이터입니다.
#[derive(Debug)]
pub struct Bone {
    /// 연결된 스키닝 데이터입니다.
    target: Arc<Skin>, 

    /// 로컬 뼈 변환 행렬 데이터입니다.
    transforms: Vec<BoneTransform>, 
}

impl Bone {
    pub fn new<I>(
        target: Arc<Skin>, 
        transforms: I
    ) -> Self 
    where 
        I: IntoIterator<Item = BoneTransform>, 
        I::IntoIter: ExactSizeIterator, 
    {
        Self { 
            target, 
            transforms: transforms.into_iter().collect() 
        }
    }

    /// 연결된 스키닝 데이터를 반환합니다.
    #[inline]
    #[must_use]
    pub fn target(&self) -> &Arc<Skin> {
        &self.target
    }

    /// 뼈 변환 행렬들을 반환합니다.
    #[inline]
    #[must_use]
    pub fn transforms<'a>(&'a self) -> impl Iterator<Item = &'a BoneTransform> + 'a {
        self.transforms.iter()
    }
}



/// 로컬 뼈 변환 행렬입니다.
#[derive(Debug, Clone)]
pub struct BoneTransform {
    pub scale: gmm::Float3, 
    pub rotation: gmm::Float4, 
    pub translation: gmm::Float3, 
}

impl BoneTransform {
    /// 뼈 변환 데이터로부터 로컬 변환 행렬을 생성하여 반환합니다.
    #[inline]
    #[must_use]
    pub fn as_matrix(&self) -> gmm::Matrix {
        gmm::Matrix::from_scale_rotation_translation(
            self.scale.into(), 
            self.rotation.into(), 
            self.translation.into()
        )
    }

    /// 두 뼈 변환 데이터를 선형보간(Linear interpolation)합니다.
    /// 
    /// 주어지는 `t`의 값은 0 ~ 1 사이의 값으로 변환되며, 1.0에 가까울수록 `other`와 같아집니다.
    /// 
    #[must_use]
    pub fn lerp(&self, other: &Self, t: f32) -> BoneTransform {
        let t = t.clamp(0.0, 1.0);
        BoneTransform { 
            scale: (1.0 - t) * self.scale + t * other.scale, 
            rotation: (1.0 - t) * self.rotation + t * other.rotation, 
            translation: (1.0 - t) * self.translation + t * other.translation 
        }
    }
}
