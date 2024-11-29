use std::ops::{Deref, DerefMut};

use hecs::Entity;
use hecs::QueryOneError;
use hecs::World;

/// ## Parent Entity
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Parent(pub Entity);

impl Deref for Parent {
    type Target = Entity;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Parent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// ## Child Entity
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Child(pub Entity);

impl Deref for Child {
    type Target = Entity;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Child {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// ## Sibling Entity
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sibling(pub Entity);

impl Deref for Sibling {
    type Target = Entity;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Sibling {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// ## Entity Not Found
/// `World`에서 해당 `Entity`를 찾지 못한 경우 발생하는 오류입니다.
#[derive(Debug, thiserror::Error)]
#[error("entity not found (ID:{0:?})")]
pub struct NoSuchEntity(Entity);

/// 주어진 `entity`에 자식 `Entity`를 추가합니다.  
/// 이미 `Child`가 존재하는 경우 `Child`의 마지막 `Sibling`으로 추가됩니다.
pub fn add_child(world: &mut World, entity: Entity, new: Entity) -> Result<(), NoSuchEntity> {
    match world.query_one_mut::<&Child>(entity) {
        Ok(&child) => add_sibling(world, *child, new),
        Err(e) => match e {
            QueryOneError::NoSuchEntity => Err(NoSuchEntity(entity)),
            QueryOneError::Unsatisfied => world
                .insert_one(entity, Child(new))
                .map_err(|_| NoSuchEntity(entity)),
        },
    }
}

/// `Child`의 마지막 `Sibling`으로 추가합니다.
fn add_sibling(world: &mut World, entity: Entity, new: Entity) -> Result<(), NoSuchEntity> {
    match world.query_one_mut::<&Sibling>(entity) {
        Ok(&sibling) => add_sibling(world, *sibling, new),
        Err(e) => match e {
            QueryOneError::NoSuchEntity => Err(NoSuchEntity(entity)),
            QueryOneError::Unsatisfied => world
                .insert_one(entity, Sibling(new))
                .map_err(|_| NoSuchEntity(entity)),
        },
    }
}

/// 주어진 `Entity`의 계층 구조를 제거합니다.  
/// `Entity`가 없는 경우 아무 동작을 수행하지 않습니다. (skip)
pub fn cleanup(world: &mut World, entity: Entity) {
    if let Ok(&child) = world.query_one_mut::<&Child>(entity) {
        cleanup(world, *child);
    }

    if let Ok(&sibling) = world.query_one_mut::<&Sibling>(entity) {
        cleanup(world, *sibling);
    }

    let _ = world.despawn(entity);
}

mod animation;
mod transform;

pub use self::animation::*;
pub use self::transform::*;
