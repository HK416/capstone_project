mod animation;
mod camera;
mod student;
mod transform;

use std::ops::{Deref, DerefMut};

use hecs::{Entity, NoSuchEntity, QueryOneError, World};

pub use self::{animation::*, camera::*, student::*, transform::*};

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

/// 주어진 `entity`에 자식 `Entity`를 추가합니다.  
/// 이미 `Child`가 존재하는 경우 `Child`의 마지막 `Sibling`으로 추가됩니다.
pub fn add_child(world: &mut World, current: Entity, new: Entity) -> Result<(), NoSuchEntity> {
    let result = world.query_one_mut::<&Child>(current).map(|child| child.0);
    match result {
        Ok(child) => add_sibling(world, current, child, new),
        Err(e) => match e {
            QueryOneError::NoSuchEntity => Err(NoSuchEntity),
            QueryOneError::Unsatisfied => {
                world.insert_one(new, Parent(current))?;
                world.insert_one(current, Child(new))?;
                Ok(())
            }
        },
    }
}

/// `Child`의 마지막 `Sibling`으로 추가합니다.
pub fn add_sibling(
    world: &mut World,
    parent: Entity,
    current: Entity,
    new: Entity,
) -> Result<(), NoSuchEntity> {
    let result = world
        .query_one_mut::<&Sibling>(current)
        .map(|sibling| sibling.0);
    match result {
        Ok(sibling) => add_sibling(world, parent, sibling, new),
        Err(e) => match e {
            QueryOneError::NoSuchEntity => Err(NoSuchEntity),
            QueryOneError::Unsatisfied => {
                world.insert_one(new, Parent(current))?;
                world.insert_one(current, Sibling(new))?;
                Ok(())
            }
        },
    }
}

/// 계층 구조를 갱신합니다.  
/// 주어진 `entity`는 `ToParentTrans`, `WorldTransform` 컴포넌트를 가져야합니다.
pub fn update_hierarchy(
    world: &mut World,
    entity: Entity,
    parent: glam::Mat4,
) -> Result<(), QueryOneError> {
    let transform = {
        type Q<'a> = (&'a ToParentTrans, &'a mut WorldTransform);
        let (local_trans, world_trans) = world.query_one_mut::<Q>(entity)?;
        world_trans.0 = parent * local_trans.0;
        world_trans.0
    };

    let result = world
        .query_one_mut::<&Sibling>(entity)
        .map(|sibling| sibling.0);
    match result {
        Ok(entity) => update_hierarchy(world, entity, parent)?,
        Err(e) => match e {
            QueryOneError::Unsatisfied => {}
            QueryOneError::NoSuchEntity => return Err(e),
        },
    };

    let result = world.query_one_mut::<&Child>(entity).map(|child| child.0);
    match result {
        Ok(entity) => update_hierarchy(world, entity, transform)?,
        Err(e) => match e {
            QueryOneError::Unsatisfied => {}
            QueryOneError::NoSuchEntity => return Err(e),
        },
    };

    Ok(())
}

/// 주어진 `Entity`의 계층 구조를 제거합니다.  
/// `Entity`가 없는 경우 아무 동작을 수행하지 않습니다. (skip)
pub fn cleanup(world: &mut World, entity: Entity) {
    let sibling = world.get::<&Sibling>(entity).ok().map(|entity| entity.0);
    if let Some(sibling) = sibling {
        cleanup(world, sibling);
    }

    let child = world.get::<&Child>(entity).ok().map(|entity| entity.0);
    if let Some(child) = child {
        cleanup(world, child);
    }

    let _ = world.despawn(entity);
}
