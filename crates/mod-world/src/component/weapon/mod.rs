use crate::objects::ObjectId;



/// 모델의 뼈 노드 중 플레이어의 무기에 대한 데이터를 저장합니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weapon {
    pub muzzle: ObjectId, 
}
