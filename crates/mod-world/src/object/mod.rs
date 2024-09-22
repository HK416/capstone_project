use std::{
    any::{Any, TypeId}, 
    fmt, 
    sync::{Arc, LockResult, Mutex, MutexGuard, Weak}
};

use mod_parallelism::collections::SkipMap;



mod element;
pub use self::element::*;



/// 게임 세상에 존재하는 모든 오브젝트입니다.
pub struct GameObject {
    /// 게임 오브젝트의 이름입니다.
    name: String, 

    /// 부모 게임 오브젝트입니다.
    parent: Mutex<Option<Weak<GameObject>>>, 

    /// 형제 게임 오브젝트입니다.
    sibling: Mutex<Option<Arc<GameObject>>>, 

    /// 자식 게임 오브젝트입니다.
    child: Mutex<Option<Arc<GameObject>>>, 

    /// 부모로 부터 변환 행렬입니다.
    to_parent_trans: Mutex<gmm::Matrix>, 

    /// 월드 변환 행렬입니다.
    world_trans: Mutex<gmm::Matrix>, 

    /// 게임 오브젝트에 연결된 요소들입니다.
    elements: SkipMap<TypeId, Box<dyn Any>>, 
}

impl GameObject {
    /// 새로운 게임 오브젝트를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new<N: Into<String>>(parent: Option<Weak<GameObject>>, name: N) -> Arc<Self> {
        Self { 
            name: name.into(), 
            parent: Mutex::new(parent), 
            sibling: Mutex::new(None), 
            child: Mutex::new(None), 
            to_parent_trans: Mutex::new(gmm::Float4x4::IDENTITY.into()), 
            world_trans: Mutex::new(gmm::Float4x4::IDENTITY.into()), 
            elements: SkipMap::new() 
        }.into()
    }

    /// 게임 오브젝트의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 게임 오브젝트의 부모 게임 오브젝트를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_parent(&self) -> Option<Weak<GameObject>> {
        self.parent.lock().unwrap().clone()
    }

    /// 게임 오브젝트의 부모 게임 오브젝트를 설정합니다.
    #[inline]
    pub fn set_parent(&self, parent: Option<Weak<GameObject>>) {
        let mut lock_guard = self.parent.lock().unwrap();
        *lock_guard = parent;
    }

    /// 게임 오브젝트의 형제 게임 오브젝트를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_sibling(&self) -> Option<Arc<GameObject>> {
        self.sibling.lock().unwrap().clone()
    }

    /// 게임 오브젝트의 형제 게임 오브젝트를 설정합니다.
    #[inline]
    pub fn set_sibling(&self, sibling: Option<Arc<GameObject>>) {
        let mut lock_guard = self.sibling.lock().unwrap();
        *lock_guard = sibling;
    }

    /// 게임 오브젝트의 자식 게임 오브젝트를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_child(&self) -> Option<Arc<GameObject>> {
        self.child.lock().unwrap().clone()
    }

    /// 게임 오브젝트의 자식 게임 오브젝트를 설정합니다.
    #[inline]
    pub fn set_child(&self, child: Option<Arc<GameObject>>) {
        let mut lock_guard = self.child.lock().unwrap();
        *lock_guard = child;
    }

    /// 게임 오브젝트의 부모로 부터 변환 행렬을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_to_parent_trans(&self) -> gmm::Matrix {
        self.to_parent_trans.lock().unwrap().clone()
    }

    /// 게임 오브젝트의 부모로 부터 변환 행렬을 설정합니다.
    #[inline]
    pub fn set_to_parent_trans<'a, F>(&'a self, func: F) 
    where F: Fn(LockResult<MutexGuard<'a, gmm::Matrix>>) {
        func(self.to_parent_trans.lock())
    }

    /// 게임 오브젝트의 월드 변환 행렬을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_world_trans(&self) -> gmm::Matrix {
        self.world_trans.lock().unwrap().clone()
    }

    /// 게임 오브젝트의 월드 변환 행렬을 설정합니다.
    #[inline]
    pub fn set_world_trans<'a, F>(&'a self, func: F) 
    where F: Fn(LockResult<MutexGuard<'a, gmm::Matrix>>) {
        func(self.world_trans.lock())
    }

    /// 게임 오브젝트에 연결된 요소를 가져옵니다.
    /// 
    /// 주어진 요소가 게임 오브젝트에 없는 경우 `None`을 반환합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn get_element<T: Element>(&self) -> Option<&T> {
        self.elements.get(TypeId::of::<T>())
            .map(|element| element.downcast_ref().unwrap())
    }

    /// 게임 오브젝트에 요소를 추가합니다.
    /// 
    /// 주어진 요소가 이미 게임 오브젝트에 존재하는 경우 이전 요소를 반환합니다.
    /// 
    pub fn add_element<T: Element>(&self, element: T) -> Option<T> {
        self.elements.insert(TypeId::of::<T>(), Box::new(element))
            .map(|element| *element.downcast().unwrap())
    }
}

impl fmt::Debug for GameObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GameObject({})", &self.name)
    }
}
