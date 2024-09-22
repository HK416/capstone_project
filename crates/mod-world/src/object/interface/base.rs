use std::sync::{Arc, Weak};



/// ### Game Object
/// 게임 세상에 존재하는 모든 오브젝트는 `GameObject`를 구현해야 합니다.
/// 
pub trait GameObject : std::fmt::Debug {
    /// 게임 오브젝트의 이름을 가져옵니다.
    fn name(&self) -> &str;

    /// 부모 게임 오브젝트를 가져옵니다.
    fn get_parent(&self) -> Option<&Weak<dyn GameObject>>;

    /// 형제 게임 오브젝트를 가져옵니다.
    fn get_sibling(&self) -> Option<&Arc<dyn GameObject>>;

    /// 자식 게임 오브젝트를 가져옵니다.
    fn get_child(&self) -> Option<&Arc<dyn GameObject>>;

    /// 부모로 부터 변환 행렬을 가져옵니다.
    fn to_parent_trans(&self) -> gmm::Matrix;

    /// 월드 변환 행렬을 가져옵니다.
    fn world_trans(&self) -> gmm::Matrix;
}
