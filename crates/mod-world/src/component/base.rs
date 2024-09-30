use super::{ArenaID, Transform};



/// 게임 세상에 존재하는 모든 오브젝트가 구현해야 하는 `trait`입니다.
pub trait GameObject {
    /// 게임 오브젝트의 식별자를 가져옵니다.
    fn id(&self) -> ArenaID;

    /// 게임 오브젝트의 이름을 가져옵니다.
    fn name(&self) -> &str;


    /// 부모 게임 오브젝트의 식별자를 가져옵니다.
    fn get_parent(&self) -> Option<ArenaID>;

    /// 부모 게임 오브젝트의 식별자를 설정합니다.
    fn set_parent(&mut self, id: Option<ArenaID>);

    /// 형제 게임 오브젝트의 식별자를 가져옵니다.
    fn get_sibling(&self) -> Option<ArenaID>;

    /// 형제 게임 오브젝트의 식별자를 설정합니다.
    fn set_sibling(&mut self, id: Option<ArenaID>);

    /// 자식 게임 오브젝트의 식별자를 가져옵니다.
    fn get_child(&self) -> Option<ArenaID>;

    /// 자식 게임 오브젝트의 식별자를 설정합니다.
    fn set_child(&mut self, id: Option<ArenaID>);


    /// 게임 오브젝트의 로컬 변환 행렬(부모로 부터 변환 행렬)을 가져옵니다.
    fn get_local_transform(&self) -> &Transform;
    
    /// 게임 오브젝트의 로컬 변환 행렬(부모로 부터 변환 행렬)을 설정합니다.
    fn set_local_transform(&mut self, transform: Transform);


    /// 게임 오브젝트의 월드 변환 행렬을 가져옵니다.
    fn get_world_transform(&self) -> &Transform;
    
    /// 게임 오브젝트의 월드 변환 행렬을 설정합니다.
    fn set_world_transform(&mut self, transform: Transform);
}

impl std::fmt::Debug for dyn GameObject {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(GameObject))
            .field("id", &self.id())
            .field("name", &self.name())
            .finish()
    }
}
