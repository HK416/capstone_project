mod camera;
mod character;
mod control;
mod physics;
mod transform;

use std::ops::{Deref, DerefMut};

use hecs::{Entity, QueryOneError, ViewBorrow, World};

pub use self::{camera::*, character::*, control::*, physics::*, transform::*};

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

/// ## Timer
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Timer(pub f32);

impl Default for Timer {
    fn default() -> Self {
        Self(0.0)
    }
}

/// 주어진 엔터티에 자식 엔터티를 추가합니다.
/// 이미 엔터티의 자식 엔터티가 존재하는 경우 자식 엔터티의 마지막 형제 엔터티로 추가됩니다.
///
/// # Panics
/// - 주어진 엔터티는 모두 유효한 엔터티여야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn add_child(world: &mut World, target_entity: Entity, new_entity: Entity) {
    let query = world.query_one_mut::<&Child>(target_entity);
    match query.cloned() {
        Ok(child_entity) => add_sibling(world, target_entity, *child_entity, new_entity),
        Err(e) => match e {
            QueryOneError::Unsatisfied => {
                world
                    .insert_one(new_entity, Parent(target_entity))
                    .expect("invalid entity");
                world
                    .insert_one(target_entity, Child(new_entity))
                    .expect("invalid entity");
            }
            _ => panic!("invalid entity"),
        },
    }
}

/// 주어진 엔터티의 형제 엔터티를 추가합니다.
/// 이미 엔터티의 형제 엔터티가 존재하는 경우 형제 엔터티의 마지막 형제 엔터티로 추가합니다.
///
/// # Panics
/// - 주어진 엔터티는 모두 유효한 엔터티여야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn add_sibling(
    world: &mut World,
    parent_entity: Entity,
    target_entity: Entity,
    new_entity: Entity,
) {
    let query = world.query_one_mut::<&Sibling>(target_entity);
    match query.cloned() {
        Ok(sibling_entity) => add_sibling(world, parent_entity, *sibling_entity, new_entity),
        Err(e) => match e {
            QueryOneError::Unsatisfied => {
                world
                    .insert_one(new_entity, Parent(parent_entity))
                    .expect("invalid entity");
                world
                    .insert_one(target_entity, Sibling(new_entity))
                    .expect("invalid entity");
            }
            _ => panic!("invalid entity"),
        },
    }
}

/// 엔터티 계층 구조를 제거합니다.
///
/// # Panics
/// 주어진 엔터티는 유효한 엔터티여야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn cleanup(world: &mut World, entity: Entity) {
    let query = world.query_one_mut::<&Sibling>(entity).ok();
    if let Some(sibling_entity) = query.cloned() {
        cleanup(world, *sibling_entity);
    }

    let query = world.query_one_mut::<&Child>(entity).ok();
    if let Some(child_entity) = query.cloned() {
        cleanup(world, *child_entity);
    }

    world.despawn(entity).expect("invalid entity");
}

/// 엔터티 계층 구조를 갱신합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 주어진 엔터티는 로컬 변환 행렬(`ToParentTrans`), 월드 변환 행렬(`WorldTransform`)을
/// 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn update_entity_hierarchy(world: &mut World, entity: Entity, parent_transform: glam::Mat4) {
    let child_view = world.view::<&Child>();
    let sibling_view = world.view::<&Sibling>();
    let mut transform_view = world.view::<(&ToParentTrans, &mut WorldTransform)>();

    update_entity_hierarchy_recursion(
        &child_view,
        &sibling_view,
        &mut transform_view,
        entity,
        parent_transform,
    );
}

/// 엔터티 계층 구조를 갱신하는 재귀함수입니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 주어진 엔터티는 로컬 변환 행렬(`ToParentTrans`), 월드 변환 행렬(`WorldTransform`)을
/// 갖고 있어야 합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
///
fn update_entity_hierarchy_recursion(
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    transform_view: &mut ViewBorrow<'_, (&ToParentTrans, &mut WorldTransform)>,
    entity: Entity,
    parent_transform: glam::Mat4,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티의 계층 구조를 갱신합니다.
    if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
        update_entity_hierarchy_recursion(
            child_view,
            sibling_view,
            transform_view,
            *sibling_entity,
            parent_transform,
        );
    }

    // 현재 엔터티의 월드 변환 행렬을 갱신합니다.
    let (local_transform, world_transform) = transform_view
        .get_mut(entity)
        .expect("invalid entity or invalid entity component");
    world_transform.0 = parent_transform * local_transform.0;

    // 자식 엔터티가 존재하는 경우 자식 엔터티의 계층 구조를 갱신합니다.
    let parent_transform = world_transform.0;
    if let Some(child_entity) = child_view.get(entity).cloned() {
        update_entity_hierarchy_recursion(
            child_view,
            sibling_view,
            transform_view,
            *child_entity,
            parent_transform,
        );
    }
}
