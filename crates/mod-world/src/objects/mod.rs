mod transform;
mod world;

pub use self::transform::*;
pub use self::world::*;



use std::{any::{Any, TypeId}, collections::HashMap};

/// 게임 오브젝트
#[derive(Debug)]
pub struct GameObject {
    /// 게임 오브젝트 식별자입니다.
    id: ObjectId, 

    /// 게임 오브젝트 이름입니다.
    pub name: String, 


    /// 부모 게임 오브젝트의 식별자입니다.
    pub parent: ObjectId, 

    /// 형제 게임 오브젝트의 식별자입니다.
    pub sibling: ObjectId, 

    /// 자식 게임 오브젝트의 식별자입니다.
    pub child: ObjectId, 


    /// 로컬 변환 행렬(부모로 부터 변환 행렬)입니다.
    pub local_transform: gmm::Matrix, 

    /// 월드 변환 행렬입니다.
    pub world_transform: gmm::Matrix, 


    /// 게임 오브젝트가 가진 요소입니다.
    elements: HashMap<TypeId, Box<dyn Any>>
}

impl GameObject {
    /// 새로운 게임 오브젝트를 생성합니다.
    #[inline]
    #[must_use]
    pub(super) fn new(id: ObjectId) -> Self {
        Self { 
            id, 
            name: "Unknown".to_string(), 
            parent: ObjectId::NIL, 
            sibling: ObjectId::NIL, 
            child: ObjectId::NIL, 
            local_transform: gmm::Matrix::IDENTITY, 
            world_transform: gmm::Matrix::IDENTITY, 
            elements: HashMap::with_capacity(32) 
        }
    }

    /// 게임 오브젝트 식별자를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    /// 게임 오브젝트에 요소를 추가합니다.
    /// 해당 요소가 존재하는 경우 요소를 교체합니다.
    #[inline]
    pub fn insert<T: 'static>(&mut self, element: T) -> Option<T> {
        self.elements.insert(TypeId::of::<T>(), Box::new(element))
            .map(|element| unsafe { *element.downcast::<T>().unwrap_unchecked() })
    }

    /// 게임 오브젝트에 요소를 제거합니다.
    /// 해당 요소가 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    #[inline]
    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.elements.remove(&TypeId::of::<T>())
            .map(|element| unsafe { *element.downcast::<T>().unwrap_unchecked() })
    }

    /// 게임 오브젝트의 요소를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.elements.get(&TypeId::of::<T>())
            .map(|element| unsafe { element.downcast_ref::<T>().unwrap_unchecked() })
    }

    /// 게임 오브젝트의 요소를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.elements.get_mut(&TypeId::of::<T>())
            .map(|element| unsafe { element.downcast_mut::<T>().unwrap_unchecked() })
    }
}
