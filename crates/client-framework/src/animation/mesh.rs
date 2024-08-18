use std::sync::Arc;

use crate::animation::BoneTransform;
use crate::render::skin::Skin;



/// 키 프레임의 스키닝된 메쉬 데이터입니다.
#[derive(Debug)]
pub struct SkinnedMesh {
    /// 연결된 스키닝 데이터입니다.
    skinning: Arc<Skin>, 

    /// 뼈 변환 행렬 데이터입니다.
    bone_transforms: Vec<BoneTransform>, 
}

impl SkinnedMesh {
    /// 키 프레임의 스키닝된 메쉬 데이터입니다.
    #[must_use]
    pub fn new<I>(skinning: Arc<Skin>, bone_transforms: I) -> Self 
    where 
        I: IntoIterator<Item = BoneTransform>, 
        I::IntoIter: ExactSizeIterator, 
    {   
        Self { 
            skinning, 
            bone_transforms: bone_transforms.into_iter().collect(), 
        }
    }

    /// 연결된 스키닝 데이터를 반환합니다.
    #[inline]
    #[must_use]
    pub fn skinning(&self) -> &Skin {
        &self.skinning
    }

    /// 각 뼈의 변환 데이터를 반환합니다.
    #[inline]
    #[must_use]
    pub fn bone_transforms<'a>(&'a self) -> impl Iterator<Item = &'a BoneTransform> + 'a {
        self.bone_transforms.iter().map(|bone_transform| bone_transform)
    }
}
