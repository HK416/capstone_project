use std::{any::{Any, TypeId}, collections::HashMap, sync::Arc};

use super::{IdGenerator, Transform, WorldID};



/// 게임 세상에 존재하는 오브젝트입니다.
pub struct GameObject {
    /// 게임 오브젝트의 식별자입니다.
    id: WorldID, 

    /// 게임 오브젝트의 이름입니다.
    name: String, 


    /// 부모 게임 오브젝트의 식별자입니다.
    parent: Option<WorldID>, 

    /// 형제 게임 오브젝트의 식별자입니다.
    sibling: Option<WorldID>, 

    /// 자식 게임 오브젝트의 식별자입니다.
    child: Option<WorldID>, 


    /// 게임 오브젝트의 로컬 변환 행렬(부모로 부터 변환 행렬)입니다.
    local_transform: Transform, 

    /// 게임 오브젝트의 월드 변환 행렬입니다.
    world_transform: Transform, 


    /// 게임 오브젝트에 연결된 요소입니다.
    elements: HashMap<TypeId, Box<dyn Any>>, 
}

impl GameObject {
    /// 새로운 게임 오브젝트를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new(
        id_generator: &Arc<IdGenerator>, 
        name: impl Into<String>, 
        parent: Option<WorldID>, 
    ) -> Self {
        Self { 
            id: id_generator.alloc(), 
            name: name.into(), 
            parent, 
            sibling: None, 
            child: None, 
            local_transform: Transform::new(), 
            world_transform: Transform::new(), 
            elements: HashMap::new() 
        }
    }

    /// 게임 오브젝트의 식별자를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn id(&self) -> &WorldID {
        &self.id
    }

    /// 게임 오브젝트의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }


    /// 부모 게임 오브젝트의 식별자를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_parent(&self) -> Option<&WorldID> {
        self.parent.as_ref()
    }

    /// 부모 게임 오브젝트의 식별자를 설정합니다.
    #[inline]
    pub fn set_parent(&mut self, id: Option<WorldID>) {
        self.parent = id;
    }

    /// 형제 게임 오브젝트의 식별자를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_sibling(&self) -> Option<&WorldID> {
        self.sibling.as_ref()
    }

    /// 형제 게임 오브젝트의 식별자를 설정합니다.
    #[inline]
    pub fn set_sibling(&mut self, id: Option<WorldID>) {
        self.sibling = id;
    }

    /// 자식 게임 오브젝트의 식별자를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_child(&self) -> Option<&WorldID> {
        self.child.as_ref()
    }

    /// 자식 게임 오브젝트의 식별자를 설정합니다.
    #[inline]
    pub fn set_child(&mut self, id: Option<WorldID>) {
        self.child = id;
    }


    /// 로컬 변환 행렬(부모로 부터 변환 행렬)을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_local_transform(&self) -> &Transform {
        &self.local_transform
    }

    /// 로컬 변환 행렬(부모로 부터 변환 행렬)을 설정합니다.
    #[inline]
    pub fn set_local_transform(&mut self, transform: impl Into<Transform>) {
        self.local_transform = transform.into();
    }

    /// 월드 변환 행렬을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_world_transform(&self) -> &Transform {
        &self.world_transform
    }

    /// 월드 변환 행렬을 설정합니다.
    #[inline]
    pub fn set_world_transform(&mut self, transform: impl Into<Transform>) {
        self.world_transform = transform.into();
    }


    /// 게임 오브젝트에 요소를 추가합니다.
    #[inline]
    pub fn insert<T: 'static>(&mut self, element: T) -> Option<T> {
        self.elements.insert(TypeId::of::<T>(), Box::new(element))
            .map(|element| unsafe {
                *element.downcast::<T>().unwrap_unchecked()
            })
    }

    /// 게임 오브젝트에 요소를 제거합니다.
    #[inline]
    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.elements.remove(&TypeId::of::<T>())
            .map(|element| unsafe {
                *element.downcast::<T>().unwrap_unchecked()
            })
    }

    /// 게임 오브젝트의 요소를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.elements.get(&TypeId::of::<T>())
            .map(|element| unsafe {
                element.downcast_ref::<T>().unwrap_unchecked()
            })
    }

    /// 게임 오브젝트의 요소를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.elements.get_mut(&TypeId::of::<T>())
            .map(|element| unsafe {
                element.downcast_mut::<T>().unwrap_unchecked()
            })
    }
}

impl std::fmt::Debug for GameObject {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(GameObject))
            .field("id", &self.id())
            .field("name", &self.name())
            .finish()
    }
}
