use std::sync::Arc;

use crate::render::mesh::SkinnedMesh;



/// 스키닝 데이터입니다.
#[derive(Debug)]
pub struct Skinning {
    /// 대상 스키닝된 메쉬입니다.
    pub skinned_mesh: Arc<SkinnedMesh>, 

    /// 뼈 오브젝트의 부모로 부터 변환 행렬입니다.
    /// 
    /// `bones`와 같은 순서를 가집니다.
    /// 
    pub transforms: Vec<gmm::Float4x4>, 
}
