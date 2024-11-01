use std::{
    any::{Any, TypeId}, 
    collections::HashMap, 
    sync::atomic::{AtomicU64, Ordering as MemOrdering}
};

use mod_parallelism::collections::{MutGuard, RefGuard, SkipMap, Values};

use crate::task::DrawTask;

use super::GameObject;



/// 게임 오브젝트 식별자
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(u64);

impl ObjectId {
    /// 비어있는 게임 오브젝트 식별자입니다.
    pub const NIL: Self = Self(0);
}

impl ObjectId {
    /// 게임 오브젝트 식별자가 비어있는지 여부를 반환합니다.
    #[inline]
    #[must_use]
    pub fn is_nil(&self) -> bool {
        *self == Self::NIL
    }
}

impl Default for ObjectId {
    #[inline]
    fn default() -> Self {
        Self::NIL
    }
}





/// 생성된 게임 오브젝트를 관리하는 게임 월드
#[derive(Debug)]
pub struct GameWorld {
    /// 게임 오브젝트 식별자를 생성하는데 사용됩니다.
    value: AtomicU64, 

    /// 생성된 게임 오브젝트 목록입니다.
    objects: SkipMap<ObjectId, GameObject>, 

    /// 게임 오브젝트 그리기 작업 집합입니다.
    draw_task: SkipMap<ObjectId, DrawTask> 
}

impl GameWorld {
    /// 새로운 게임 세상을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { 
            value: AtomicU64::new(1), 
            objects: SkipMap::new(), 
            draw_task: SkipMap::new() 
        }
    }

    /// 게임 세상에 게임 오브젝트를 생성합니다.
    /// 게임 오브젝트 식별자를 반환합니다.
    pub fn spawn(&self, desc: GameObjectDescriptor) -> ObjectId {
        let value = self.value.fetch_add(1, MemOrdering::AcqRel);
        let id = ObjectId(value);

        let mut object = GameObject::new(id);
        object.name = desc.name;
        object.parent = desc.parent;
        object.sibling = desc.sibling;
        object.child = desc.child;
        object.local_transform = desc.local_transform;
        object.world_transform = desc.world_transform;
        object.elements = desc.elements;

        self.objects.insert(id, object);

        id
    }

    /// 주어진 식별자에 해당하는 게임 오브젝트를 게임 세상에서 제거합니다.
    /// 해당 게임 오브젝트가 게임 세상에 존재하지 않는 경우 아무 행동을 하지 않습니다.
    #[inline]
    pub fn despawn(&self, id: &ObjectId) {
        self.objects.remove(id);
        self.draw_task.remove(id);
    }

    /// 주어진 식별자에 해당하는 게임 오브젝트가 게임 세상에 포함되어 있는지 여부를 반환합니다.
    #[inline]
    #[must_use]
    pub fn contains(&self, id: &ObjectId) -> bool {
        self.objects.contains_key(id)
    }

    /// 주어진 식별자에 해당하는 게임 오브젝트를 게임 세상에서 가져옵니다.
    /// 해당 게임 오브젝트가 게임 세상에 존재하지 않는 경우 `None`을 반환합니다.
    #[inline]
    #[must_use]
    pub fn get(&self, id: &ObjectId) -> Option<RefGuard<'_, ObjectId, GameObject>> {
        self.objects.get(id)
    }

    /// 주어진 식별자에 해당하는 게임 오브젝트를 게임 세상에서 가져옵니다.
    /// 해당 게임 오브젝트가 게임 세상에 존재하지 않는 경우 `None`을 반환합니다.
    #[inline]
    #[must_use]
    pub fn get_mut(&self, id: &ObjectId) -> Option<MutGuard<'_, ObjectId, GameObject>> {
        self.objects.get_mut(id)
    }
}

impl GameWorld {
    /// 게임 오브젝트 그리기 작업을 등록합니다.
    /// 이미 해당 오브젝트의 그리기 작업이 존재하는 경우 새로운 작업으로 교체됩니다.
    /// 
    /// # Errors
    /// 그리기 작업의 대상 게임 오브젝트가 게임 세상에 존재하지 않을 경우
    /// `TaskRegistError`를 반환합니다.
    /// 
    #[inline]
    pub fn regist_draw_task(&self, task: DrawTask) -> Result<Option<DrawTask>, TaskRegistError> {
        if !self.objects.contains_key(&task.id()) {
            return Err(TaskRegistError::NoSuchObject(task.id()));
        }
        Ok(self.draw_task.insert(task.id(), task))
    }

    /// 게임 오브젝트 그리기 작업을 등록 해제합니다.
    /// 해당 오브젝트가 존재하지 않거나, 해당 오브젝트의 그리기 작업이 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    #[inline]
    pub fn unregist_draw_task(&self, id: &ObjectId) -> Option<DrawTask> {
        self.draw_task.remove(id)
    }

    /// 그리기 작업 목록을 순회하는 반복자를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn draw_tasks(&self) -> Values<'_, ObjectId, DrawTask> {
        self.draw_task.values()
    }
}





/// 작업을 등록하는 도중 발생할 수 있는 오류 목록입니다.
#[derive(Debug, thiserror::Error)]
pub enum TaskRegistError {
    /// 게임 세상에서 오브젝트를 찾을 수 없는 경우 발생하는 오류입니다.
    #[error("Game object not found in game world! ({0:?})")]
    NoSuchObject(ObjectId), 
}




/// 게임 오브젝트 설명자
#[derive(Debug)]
pub struct GameObjectDescriptor {
    /// 게임 오브젝트의 이름입니다.
    pub name: String, 

    /// 부모 게임 오브젝트 식별자입니다.
    pub parent: ObjectId, 

    /// 형제 게임 오브젝트 식별자입니다.
    pub sibling: ObjectId, 

    /// 자식 게임 오브젝트 식별자입니다.
    pub child: ObjectId, 

    /// 로컬 변환 행렬(부모로 부터 변환 행렬)입니다.
    pub local_transform: gmm::Matrix, 

    /// 월드 변환 행렬입니다.
    pub world_transform: gmm::Matrix, 

    /// 게임 오브젝트가 가진 요소입니다.
    elements: HashMap<TypeId, Box<dyn Any>>
}

impl GameObjectDescriptor {
    /// 새로운 게임 오브젝트 설명자를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 게임 오브젝트 이름을 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 부모 게임 오브젝트 식별자를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_parent(mut self, parent: ObjectId) -> Self {
        self.parent = parent;
        self
    }

    /// 형제 게임 오브젝트 식별자를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_sibling(mut self, sibling: ObjectId) -> Self {
        self.sibling = sibling;
        self
    }

    /// 자식 게임 오브젝트 식별자를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_child(mut self, child: ObjectId) -> Self {
        self.child = child;
        self
    }

    /// 로컬 변환 행렬을 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_local_transform(mut self, transform: impl Into<gmm::Matrix>) -> Self {
        self.local_transform = transform.into();
        self
    }

    /// 월드 변환 행렬을 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_world_transform(mut self, transform: impl Into<gmm::Matrix>) -> Self {
        self.world_transform = transform.into();
        self
    }

    /// 게임 요소를 추가합니다.
    #[inline]
    #[must_use]
    pub fn with_element<T: 'static>(mut self, element: T) -> Self {
        self.elements.insert(TypeId::of::<T>(), Box::new(element));
        self
    }
}

impl Default for GameObjectDescriptor {
    #[inline]
    fn default() -> Self {
        Self { 
            name: "Unknown".to_string(), 
            parent: ObjectId::NIL, 
            sibling: ObjectId::NIL, 
            child: ObjectId::NIL, 
            local_transform: gmm::Matrix::IDENTITY, 
            world_transform: gmm::Matrix::IDENTITY, 
            elements: HashMap::with_capacity(32) 
        }
    }
}
