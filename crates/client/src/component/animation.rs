use ahash::{HashMap, HashSet};
use hecs::Entity;

/// ## Skinning Animation
/// 스키닝 애니메이션에 사용되는 스키닝 메쉬 엔터티와 최상위 뼈 노드 엔터티의 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkinningAnimation {
    /// NOTE: `BoneCollection`의 `root`와 다름!
    pub root: Entity,
    pub meshes: HashMap<String, Entity>,
    pub lower_nodes: HashSet<Entity>,
}

/// ## Bone Collection
/// 스키닝된 메쉬를 구성하는 뼈의 엔터티 모음입니다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoneCollection {
    /// NOTE: 스키닝된 메쉬의 최상위 뼈를 나타냅니다.
    pub root: Entity,
    pub bones: Vec<Entity>,
}
