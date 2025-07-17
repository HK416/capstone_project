use hecs::Entity;
use mod_physics::object3d::{Frustum, Sphere};

/// 스테이지의 BVH입니다.
#[derive(Debug, Clone)]
pub struct StageBoundingVolumnHierarchy {
    pub area: Vec<Entity>,
    pub root: Option<Box<StageBoundingVolumn>>,
}

impl Default for StageBoundingVolumnHierarchy {
    fn default() -> Self {
        Self {
            area: Vec::default(),
            root: None,
        }
    }
}

/// 스테이지를 구성하는 오브젝트의 Bounding Volumn입니다.
#[derive(Debug, Clone)]
pub struct StageBoundingVolumn {
    pub entity: Entity,
    pub sphere: Sphere,
    pub left: Option<Box<StageBoundingVolumn>>,
    pub right: Option<Box<StageBoundingVolumn>>,
}

/// 스테이지 엔터티 계층 구조에 대해 카메라 뷰 프러스텀 컬링을 수행합니다.
///
/// # Note
/// 이 함수는 카메라의 월드 변환 행렬을 갱신한 후 호출되어야 합니다.
///
pub fn cull_stage_entities(
    frustum: &Frustum,
    hierarchy: &StageBoundingVolumnHierarchy,
) -> Vec<Entity> {
    let mut entity_list = hierarchy.area.clone();
    if let Some(current) = &hierarchy.root {
        cull_state_entity(frustum, current, &mut entity_list);
    }
    entity_list
}

/// 스테이지 엔터티에 대해 카메라 뷰 프러스텀 컬링을 수행합니다.
fn cull_state_entity(
    frustum: &Frustum,
    current: &StageBoundingVolumn,
    entity_list: &mut Vec<Entity>,
) {
    if frustum.sphere_test(&current.sphere) {
        entity_list.push(current.entity);
    }
    if let Some(current) = &current.left {
        cull_state_entity(frustum, current, entity_list);
    }
    if let Some(current) = &current.right {
        cull_state_entity(frustum, current, entity_list);
    }
}
