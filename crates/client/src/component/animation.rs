use hecs::Entity;

/// ## Bone Collection
/// 스키닝된 메쉬를 구성하는 뼈의 `Entity` 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoneCollection {
    pub root: Entity,
    pub bones: Vec<Entity>,
}
