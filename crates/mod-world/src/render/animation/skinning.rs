use std::sync::Arc;

use crate::{
    component::WorldID, 
    render::mesh::{BoneMatrixUniform, DynamicMeshUniform}
};



/// 스키닝된 메쉬 정보
#[derive(Debug)]
pub struct SkinnedMeshInfo {
    /// 최상위 뼈 노드의 식별자입니다.
    pub root_bone: WorldID, 

    /// 스키닝 데이터를 구성하는 뼈 노드들의 식별자입니다.
    pub bones: Vec<WorldID>, 

    /// 스키닝된 메쉬 데이터 유니폼 버퍼입니다.
    pub mesh_uniform: DynamicMeshUniform, 

    /// 뼈 바인드 포즈 데이터 유니폼 버퍼입니다.
    /// 
    /// `bones`와 같은 순서를 가집니다.
    /// 
    pub bindpose_uniform: BoneMatrixUniform, 

    /// 뼈 변환 행렬 데이터 유니폼 버퍼입니다.
    /// 
    /// `bones`와 같은 순서를 가집니다.
    /// 
    pub bone_transform_uniform: BoneMatrixUniform, 
}



/// 스키닝 데이터입니다.
#[derive(Debug)]
pub struct SkinningData {
    /// 스키닝된 메쉬 정보입니다.
    pub mesh: Arc<SkinnedMeshInfo>, 

    /// 뼈 오브젝트의 부모로 부터 변환 행렬입니다.
    pub transforms: Vec<gmm::Matrix>, 
}
